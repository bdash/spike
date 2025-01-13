pub mod bus;
pub mod node;

use binrw::BinRead;

use crate::Result;

pub trait Response: Sized + std::fmt::Debug {
    fn from_bytes(bytes: &[u8]) -> Result<Self>;

    fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

pub trait Message: std::fmt::Debug {
    type Response: Response;

    fn serialize(&self) -> Vec<u8>;
}

impl<T: BinRead + std::fmt::Debug> Response for T
where
    for<'a> <T as BinRead>::Args<'a>: Default,
{
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        assert!(bytes.len() == Self::size());
        Ok(Self::read_le(&mut std::io::Cursor::new(bytes))?)
    }
}
