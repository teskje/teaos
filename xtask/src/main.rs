use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use std::{env, io};

use anyhow::{Context, bail};
use aws_sdk_ebs::primitives::ByteStream;
use aws_sdk_ebs::types::ChecksumAlgorithm;
use aws_sdk_ec2::client::Waiters;
use aws_sdk_ec2::types::builders::{
    BlockDeviceMappingBuilder, EbsBlockDeviceBuilder, FilterBuilder, TagBuilder,
    TagSpecificationBuilder,
};
use aws_sdk_ec2::types::{ArchitectureValues, BootModeValues, InstanceType, ResourceType};
use base64::prelude::*;
use fatfs::{FileSystem, FormatVolumeOptions, FsOptions};
use fscommon::{BufStream, StreamSlice};
use gpt::mbr::ProtectiveMBR;
use gpt::{GptConfig, partition_types};
use sha2::{Digest, Sha256};

/// xtask runner for the TeaOS repo.
#[derive(argh::FromArgs)]
struct Args {
    #[argh(subcommand)]
    task: TaskArgs,
}

#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum TaskArgs {
    Qemu(QemuArgs),
    Aws(AwsArgs),
}

/// Run TeaOS in qemu.
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "qemu")]
struct QemuArgs {
    /// build in release mode
    #[argh(switch)]
    release: bool,
    /// wait for a gdb connection on tcp::1234
    #[argh(switch)]
    gdb: bool,
}

/// Run TeaOS in AWS.
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "aws")]
struct AwsArgs {
    /// build in release mode
    #[argh(switch)]
    release: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();

    let repo_root = get_repo_root()?;
    env::set_current_dir(repo_root)?;

    match args.task {
        TaskArgs::Qemu(args) => task_qemu(args.release, args.gdb),
        TaskArgs::Aws(args) => task_aws(args.release).await,
    }
}

fn task_qemu(release: bool, gdb: bool) -> anyhow::Result<()> {
    let disk_img = build_disk_image(release)?;

    let mut cmd = Command::new("qemu-system-aarch64");
    cmd.args(["-machine", "virt"])
        .args(["-cpu", "neoverse-n1"])
        .args(["-m", "512M"])
        .args([
            "-drive",
            "if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
        ])
        .args(["-drive", &format!("format=raw,file={}", disk_img.display())])
        .arg("-nographic");
    if gdb {
        cmd.args(["-s", "-S"]);
        println!("qemu waits for gdb; connect with `target remote localhost:1234`");
    }
    cmd.status().context("qemu-system-aarch64")?;

    Ok(())
}

async fn task_aws(release: bool) -> anyhow::Result<()> {
    let disk_img = build_disk_image(release)?;

    let aws_config = aws_config::load_from_env().await;
    let ec2 = aws_sdk_ec2::Client::new(&aws_config);
    let ebs = aws_sdk_ebs::Client::new(&aws_config);

    println!("creating EBS snapshot");
    let snapshot_id = create_ebs_snapshot(&ebs, &disk_img).await?;

    println!("waiting for snapshot to complete (snapshot_id={snapshot_id})");
    ec2.wait_until_snapshot_completed()
        .snapshot_ids(&snapshot_id)
        .wait(Duration::from_secs(600))
        .await?;

    println!("checking for existing AMI");
    let filter = FilterBuilder::default()
        .name("name")
        .values("TeaOS")
        .build();
    let output = ec2.describe_images().filters(filter).send().await?;
    let image_id = output
        .images
        .and_then(|mut i| i.pop())
        .and_then(|i| i.image_id);

    if let Some(image_id) = image_id {
        println!("deregistering existing AMI (image_id={image_id})");
        ec2.deregister_image().image_id(image_id).send().await?;
    }

    println!("registering AMI (snapshot_id={snapshot_id})");
    let ebs_block_device = EbsBlockDeviceBuilder::default()
        .snapshot_id(snapshot_id)
        .build();
    let block_device_mapping = BlockDeviceMappingBuilder::default()
        .device_name("/dev/sda1")
        .ebs(ebs_block_device)
        .build();
    let output = ec2
        .register_image()
        .name("TeaOS")
        .architecture(ArchitectureValues::Arm64)
        .virtualization_type("hvm")
        .boot_mode(BootModeValues::Uefi)
        .root_device_name("/dev/sda1")
        .block_device_mappings(block_device_mapping)
        .ena_support(true)
        .send()
        .await?;
    let image_id = output.image_id.unwrap();

    println!("running EC2 instance (image_id={image_id})");
    let name_tag = TagBuilder::default().key("Name").value("TeaOS").build();
    let tag_spec = TagSpecificationBuilder::default()
        .resource_type(ResourceType::Instance)
        .tags(name_tag)
        .build();
    let output = ec2
        .run_instances()
        .image_id(image_id)
        .instance_type(InstanceType::T4gNano)
        .tag_specifications(tag_spec)
        .min_count(1)
        .max_count(1)
        .send()
        .await?;
    let instance_id = output
        .instances
        .and_then(|mut i| i.pop())
        .and_then(|i| i.instance_id)
        .unwrap();

    println!("spawned EC2 instance (instance_id={instance_id})");

    Ok(())
}

fn get_repo_root() -> anyhow::Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("git rev-parse")?;

    let mut path = String::from_utf8(output.stdout)?;
    path.truncate(path.trim_end().len());

    Ok(path.into())
}

fn target_dir() -> PathBuf {
    PathBuf::from("target")
}

fn build_disk_image(release: bool) -> anyhow::Result<PathBuf> {
    println!("building boot.efi (release={release})");
    let boot_bin = build_boot(release)?;
    println!("building kernel (release={release})");
    let kernel_bin = build_kernel(release)?;
    println!("building userimg (release={release}");
    let userimg_bin = build_userimg(release)?;

    println!("creating disk image");
    let esp_img = target_dir().join("esp.img");
    create_esp_image(&esp_img, &boot_bin, &kernel_bin, &userimg_bin)?;

    Ok(esp_img)
}

fn build_boot(release: bool) -> anyhow::Result<PathBuf> {
    const TARGET: &str = "aarch64-unknown-uefi";

    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--bin", "boot", "--target", TARGET]);

    if release {
        cmd.arg("--release");
    }

    let status = cmd.status().context("cargo build")?;
    if !status.success() {
        bail!("bootloader build failed");
    }

    let profile = if release { "release" } else { "debug" };
    let mut bin_path = target_dir();
    bin_path.extend([TARGET, profile, "boot.efi"]);

    Ok(bin_path)
}

fn build_kernel(release: bool) -> anyhow::Result<PathBuf> {
    const TARGET: &str = "aarch64-unknown-none-softfloat";

    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--bin", "kernel", "--target", TARGET]);

    if release {
        cmd.arg("--release");
    }

    let status = cmd.status().context("cargo build")?;
    if !status.success() {
        bail!("kernel build failed");
    }

    let profile = if release { "release" } else { "debug" };
    let mut bin_path = target_dir();
    bin_path.extend([TARGET, profile, "kernel"]);

    Ok(bin_path)
}

fn build_userimg(release: bool) -> anyhow::Result<PathBuf> {
    const TARGET: &str = "aarch64-unknown-none-softfloat";

    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--bin", "user", "--target", TARGET]);

    if release {
        cmd.arg("--release");
    }

    let status = cmd.status().context("cargo build")?;
    if !status.success() {
        bail!("user build failed");
    }

    let profile = if release { "release" } else { "debug" };
    let mut bin_path = target_dir();
    bin_path.extend([TARGET, profile, "user"]);

    Ok(bin_path)
}

fn create_esp_image(
    img_path: &Path,
    boot_bin: &Path,
    kernel_bin: &Path,
    userimg_bin: &Path,
) -> anyhow::Result<()> {
    const MB: u64 = 1024 * 1024;
    const DISK_SIZE: u64 = 100 * MB;
    const PART_SIZE: u64 = 99 * MB;

    // Create the image file, replacing it if it already exists.
    let mut img_file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(img_path)?;

    // File the image file with `DISK_SIZE` zero bytes.
    img_file.set_len(DISK_SIZE)?;
    img_file.sync_data()?;

    // Create a protective MBR.
    let mbr = ProtectiveMBR::new();
    mbr.overwrite_lba0(&mut img_file)?;

    // Partition the image using GPT.
    let mut disk = GptConfig::new().writable(true).create(img_path)?;
    let block_size = disk.logical_block_size().as_u64();
    let part_id = disk.add_partition("EFI", PART_SIZE, partition_types::EFI, 0, None)?;
    let part_info = disk.partitions()[&part_id].clone();
    disk.write()?;

    // Build a reader for the EFI partition.
    let start_offset = part_info.first_lba * block_size;
    let end_offset = (part_info.last_lba + 1) * block_size;
    let partition = StreamSlice::new(img_file, start_offset, end_offset)?;
    let mut partition = BufStream::new(partition);

    // Format the EFI partition as FAT32.
    fatfs::format_volume(&mut partition, FormatVolumeOptions::new())?;

    // Copy the binaries into the EFI partition.
    let fs = FileSystem::new(&mut partition, FsOptions::new())?;
    let root = fs.root_dir();
    root.create_dir("efi")?;
    root.create_dir("efi/boot")?;

    let mut src = File::open(boot_bin)?;
    let mut dst = root.create_file("efi/boot/bootaa64.efi")?;
    io::copy(&mut src, &mut dst)?;

    let mut src = File::open(kernel_bin)?;
    let mut dst = root.create_file("kernel")?;
    io::copy(&mut src, &mut dst)?;

    let mut src = File::open(userimg_bin)?;
    let mut dst = root.create_file("userimg")?;
    io::copy(&mut src, &mut dst)?;

    Ok(())
}

async fn create_ebs_snapshot(ebs: &aws_sdk_ebs::Client, img_path: &Path) -> anyhow::Result<String> {
    let img_file = File::open(img_path)?;
    let img_size = img_file.metadata()?.len();
    let volume_size = img_size.div_ceil(1 << 30);

    let output = ebs
        .start_snapshot()
        .volume_size(volume_size as i64)
        .description("TeaOS img")
        .send()
        .await?;

    let snapshot_id = output.snapshot_id.unwrap();
    let block_size = output.block_size.unwrap();

    let empty_block_hash = Sha256::digest(vec![0; block_size as usize]);

    let mut block_idx = 0;
    let mut changed_blocks = 0;
    with_chunks(img_file, block_size as usize, async |block| {
        let hash = Sha256::digest(block);
        if hash != empty_block_hash {
            let checksum = BASE64_STANDARD.encode(hash);
            let data = ByteStream::from(block.to_vec());

            ebs.put_snapshot_block()
                .snapshot_id(&snapshot_id)
                .block_index(block_idx)
                .block_data(data)
                .data_length(block_size)
                .checksum(checksum)
                .checksum_algorithm(ChecksumAlgorithm::ChecksumAlgorithmSha256)
                .send()
                .await?;

            changed_blocks += 1;
        }

        block_idx += 1;
        Ok(())
    })
    .await?;

    ebs.complete_snapshot()
        .snapshot_id(&snapshot_id)
        .changed_blocks_count(changed_blocks)
        .send()
        .await?;

    Ok(snapshot_id)
}

async fn with_chunks(
    mut rd: impl Read,
    chunk_size: usize,
    mut f: impl AsyncFnMut(&[u8]) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut buf = vec![0; chunk_size];
    let mut fill = 0;
    loop {
        while fill < chunk_size {
            let len = rd.read(&mut buf[fill..])?;
            if len == 0 {
                break;
            }
            fill += len;
        }

        if fill == 0 {
            break;
        }
        if fill < chunk_size {
            buf[fill..].fill(0);
        }

        f(&buf).await?;
        fill = 0;
    }

    Ok(())
}
