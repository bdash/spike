use binrw::BinRead;
use bitvec::array::BitArray;

use super::{Message, Response};

use crate::Result;

pub trait NodeMessage: std::fmt::Debug {
    type Response: Response;

    fn serialize(&self) -> Vec<u8>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToNode<T: NodeMessage>(pub u8, pub T);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FromNode<T: Response>(pub T, pub u8, pub u8);

impl<T: Response> Response for FromNode<T> {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if std::mem::size_of::<T>() > 0 {
            let (response, trailer) = bytes.split_at(bytes.len() - 2);
            assert!(trailer.len() == 2);
            Ok(Self(T::from_bytes(response)?, trailer[0], trailer[1]))
        } else {
            Ok(Self(T::from_bytes(bytes)?, 0, 0))
        }
    }

    fn size() -> usize {
        let underlying_size = std::mem::size_of::<T>();
        if underlying_size > 0 {
            std::mem::size_of::<Self>()
        } else {
            underlying_size
        }
    }
}

fn checksum(data: &[u8]) -> u8 {
    let mut result: u64 = 0;
    for byte in data {
        result += *byte as u64;
    }
    ((256 - (result % 256)) % 256) as u8
}

impl<T: NodeMessage> Message for ToNode<T>
where
    T::Response: BinRead,
{
    type Response = FromNode<T::Response>;

    fn serialize(&self) -> Vec<u8> {
        let id = 0x80 | self.0;
        let mut message = vec![id, 0];
        message.extend(self.1.serialize());
        message[1] = (message.len() - 1) as u8;
        message.push(checksum(&message));
        message.push(Self::Response::size() as u8);
        message
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reset;

impl NodeMessage for Reset {
    type Response = ();

    fn serialize(&self) -> Vec<u8> {
        vec![0xf1]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetNodeVersion;

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetNodeVersionResponse(pub [u8; 11]);

impl NodeMessage for GetNodeVersion {
    type Response = GetNodeVersionResponse;

    fn serialize(&self) -> Vec<u8> {
        vec![0xfe]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SetTrafficFlags(pub u8);

impl NodeMessage for SetTrafficFlags {
    type Response = ();

    fn serialize(&self) -> Vec<u8> {
        vec![0xf0, self.0]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetNodeStatus;

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetNodeStatusResponse(pub [u8; 0x8]);

impl NodeMessage for GetNodeStatus {
    type Response = GetNodeStatusResponse;

    fn serialize(&self) -> Vec<u8> {
        vec![0xff]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SetLEDsAndInputs(pub u8, pub u8);

impl NodeMessage for SetLEDsAndInputs {
    type Response = ();

    fn serialize(&self) -> Vec<u8> {
        vec![0x14, self.0, 0x0, self.1, 0x0]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetInputState;

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetInputStateResponse(pub [u8; 0x8], u8, u8);

impl GetInputStateResponse {
    pub fn switches(&self) -> BitArray<[u8; 8]> {
        return BitArray::new(self.0);
    }
}

impl NodeMessage for GetInputState {
    type Response = GetInputStateResponse;

    fn serialize(&self) -> Vec<u8> {
        vec![0x11]
    }
}