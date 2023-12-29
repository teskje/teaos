use core::ops::Range;

use ark_dtb::Token;

pub(crate) struct DeviceTree<'d> {
    parser: ark_dtb::Parser<'d>,
}

impl<'d> DeviceTree<'d> {
    pub fn new(data: &'d [u8]) -> Self {
        let parser =
            ark_dtb::Parser::new(data).unwrap_or_else(|e| panic!("error parsing DTB: {e}"));

        Self { parser }
    }

    pub fn memory_reservations(&self) -> impl Iterator<Item = Range<usize>> + '_ {
        self.parser
            .memory_reservations()
            .unwrap_or_else(|e| panic!("error parsing DTB memory reservations: {e}"))
            .map(|r| {
                r.unwrap_or_else(|e| panic!("error parsing DTB memory reservation entry: {e}"))
            })
    }

    pub fn tokens(&self) -> impl Iterator<Item = Token<'d>> + '_ {
        self.parser
            .tokens()
            .unwrap_or_else(|e| panic!("error parsing DTB memory reservations: {e}"))
            .map(|r| {
                r.unwrap_or_else(|e| panic!("error parsing DTB memory reservation entry: {e}"))
            })
    }

    pub fn find_memory(&self) -> Range<usize> {
        0..0
    }
}
