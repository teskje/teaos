use alloc::vec::Vec;

pub struct String {
    chars: Vec<u16>,
}

impl String {
    pub fn as_ptr(&self) -> *const u16 {
        self.chars.as_ptr()
    }
}

impl From<&str> for String {
    fn from(s: &str) -> Self {
        let chars = s.encode_utf16().chain([0]).collect();
        Self { chars }
    }
}
