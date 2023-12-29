use core::fmt;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidMagic(u32),
    UnsupportedVersion(u32),
    NotEnoughData,
    MissingNulTerminator,
    InvalidToken(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;

        match self {
            InvalidMagic(v) => write!(f, "invalid magic: {v:#x}"),
            UnsupportedVersion(v) => write!(f, "unsupported version: {v:#x}"),
            NotEnoughData => f.write_str("not enough data"),
            MissingNulTerminator => f.write_str("missing string NUL terminator"),
            InvalidToken(v) => write!(f, "invalid token type: {v:#x}"),
        }
    }
}
