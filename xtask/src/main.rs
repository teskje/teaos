use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use anyhow::Context;

const UEFI_TARGET: &str = "aarch64-unknown-uefi";

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
}

/// Run TeaOS in qemu.
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "qemu")]
struct QemuArgs {
    /// build in release mode
    #[argh(switch)]
    release: bool,
}

fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();

    let repo_root = get_repo_root()?;
    env::set_current_dir(repo_root)?;

    match args.task {
        TaskArgs::Qemu(args) => task_qemu(args.release),
    }
}

fn task_qemu(release: bool) -> anyhow::Result<()> {
    let efi_bin = cargo_build(release)?;

    let esp_dir = target_dir().join("esp");
    let boot_dir = esp_dir.join("efi/boot");
    let boot_bin = boot_dir.join("bootaa64.efi");

    let _ = fs::remove_dir_all(&esp_dir);
    fs::create_dir_all(&boot_dir)?;
    fs::copy(efi_bin, boot_bin)?;

    Command::new("qemu-system-aarch64")
        .args(["-machine", "virt"])
        .args(["-cpu", "neoverse-n1"])
        .args(["-m", "512M"])
        .args([
            "-drive",
            "if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
        ])
        .args([
            "-drive",
            &format!("format=raw,file=fat:rw:{}", esp_dir.display()),
        ])
        .arg("-nographic")
        .status()
        .context("qemu-system-aarch64")?;

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

fn cargo_build(release: bool) -> anyhow::Result<PathBuf> {
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--bin", "teaos", "--target", UEFI_TARGET]);

    if release {
        cmd.arg("--release");
    }

    cmd.status().context("cargo build")?;

    let profile = if release { "release" } else { "debug" };
    let mut bin_path = target_dir();
    bin_path.extend([UEFI_TARGET, profile, "teaos.efi"]);

    Ok(bin_path)
}
