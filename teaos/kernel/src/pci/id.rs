pub(super) fn vendor(id: u16) -> Option<&'static str> {
    match id {
        0x1af4 => Some("Red Hat"),
        0x1b36 => Some("Red Hat"),
        0x1d0f => Some("Amazon"),
        _ => None,
    }
}

pub(super) fn device(vendor_id: u16, id: u16) -> Option<&'static str> {
    match (vendor_id, id) {
        (0x1b36, 0x0008) => Some("QEMU PCIe host bridge"),
        (0x1af4, 0x1000) => Some("Virtio network device"),
        (0x1af4, 0x1001) => Some("Virtio block device"),
        (0x1d0f, 0x0200) => Some("PCIe host bridge"),
        (0x1d0f, 0x8061) => Some("NVMe EBS Controller"),
        (0x1d0f, 0x8250) => Some("Serial device"),
        (0x1d0f, 0xec20) => Some("Elastic Network Adapter (ENA)"),
        _ => None,
    }
}

pub(super) fn class(class: u8, subclass: u8, prog_if: u8) -> Option<&'static str> {
    match (class, subclass, prog_if) {
        (0x1, 0x0, 0x0) => Some("SCSI controller"),
        (0x1, 0x8, 0x2) => Some("NVMe I/O controller"),
        (0x2, 0x0, 0x0) => Some("Ethernet controller"),
        (0x6, 0x0, 0x0) => Some("Host bridge"),
        (0x7, 0x0, 0x3) => Some("16650-compatible serial controller"),
        _ => None,
    }
}
