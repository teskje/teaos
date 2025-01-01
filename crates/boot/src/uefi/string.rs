use alloc::vec::Vec;

pub struct String {
    chars: Vec<u16>,
}

impl String {
    pub fn as_ptr(&self) -> *const u16 {
        self.chars.as_ptr()
    }
}

impl From<&[u8]> for String {
    fn from(s: &[u8]) -> Self {
        let mut chars = Vec::with_capacity(s.len() + 1);
        for &byte in s {
            chars.push(byte.into());
        }
        chars.push(0);

        Self { chars }
    }
}
