//! Known vendor and device names, according to https://pcilookup.com.

pub(super) fn vendor(id: u16) -> Option<&'static str> {
    match id {
        0x1af4 => Some("Red Hat, Inc."),
        0x1b36 => Some("Red Hat, Inc."),
        0x1d0f => Some("Amazon.com, Inc."),
        _ => None,
    }
}

pub(super) fn device(vendor_id: u16, id: u16) -> Option<&'static str> {
    match (vendor_id, id) {
        (0x1b36, 0x0008) => Some("QEMU PCIe Host bridge"),
        (0x1af4, 0x1000) => Some("Virtio network device"),
        (0x1af4, 0x1001) => Some("Virtio block device"),
        (0x1d0f, 0x8061) => Some("NVMe EBS Controller"),
        (0x1d0f, 0xec20) => Some("Elastic Network Adapter (ENA)"),
        _ => None,
    }
}
