#! /bin/bash
set -euxo pipefail

efi_bin="$1"
s3_bucket="$2"

aws="$(dirname "$0")"

rm -f "$aws/root.img*"
dd if=/dev/zero of="$aws/root.img" bs=1M count=100

gdisk "$aws/root.img" <<EOF
n
1


EF00
w
Y
EOF

disk_dev="$(
    hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$aws/root.img" \
    | head -1 | cut -f1 | xargs
)"

newfs_msdos -F 32 -v ESP "${disk_dev}s1"

mkdir -p "$aws/esp"
mount -t msdos /dev/disk4s1 "$aws/esp"
mkdir -p "$aws/esp/efi/boot"
cp "$efi_bin"  "$aws/esp/efi/boot/bootaa64.efi"
dot_clean "$aws/esp"
umount "$aws/esp"

hdiutil detach "$disk_dev"

aws s3 cp "$aws/root.img" "s3://$s3_bucket/"
task_id=$(
    aws ec2 import-snapshot \
        --description "TeaOS root volume" \
        --disk-container "Format=RAW,UserBucket={S3Bucket=$s3_bucket,S3Key=root.img}" \
    | jq -r ".ImportTaskId"
)

echo "Waiting for snapshot import to complete"
while true; do
    status=$(
        aws ec2 describe-import-snapshot-tasks --import-task-ids "$task_id" \
        | jq -r ".ImportSnapshotTasks[0].SnapshotTaskDetail.Status"
    )
    if [[ "$status" == "completed" ]]; then
        break
    fi
    sleep 10
done

snapshot_id=$(
    aws ec2 describe-import-snapshot-tasks --import-task-ids "$task_id" \
    | jq -r ".ImportSnapshotTasks[0].SnapshotTaskDetail.SnapshotId"
)

aws ec2 register-image \
    --name "TeaOS" \
    --architecture arm64 \
    --virtualization-type hvm \
    --boot-mode uefi \
    --root-device-name /dev/sda1 \
    --block-device-mappings "DeviceName=/dev/sda1,Ebs={SnapshotId=$snapshot_id}" \
    --ena-support
