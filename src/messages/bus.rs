use binrw::BinRead;

use super::Message;


#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Poll;

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PollResponse(pub u8);

impl Message for Poll {
    type Response = PollResponse;

    fn serialize(&self) -> Vec<u8> {
        vec![0x0]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SetPower(pub bool);

impl Message for SetPower {
    type Response = ();

    fn serialize(&self) -> Vec<u8> {
        vec![0x07, 1, self.0 as u8]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SetAmpPower(pub bool);

impl Message for SetAmpPower {
    type Response = ();

    fn serialize(&self) -> Vec<u8> {
        vec![0x08, 1, self.0 as u8]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetBridgeState;

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetBridgeStateResponse(u8, u8);

impl Message for GetBridgeState {
    type Response = GetBridgeStateResponse;

    fn serialize(&self) -> Vec<u8> {
        vec![0x0a, 0x0]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetBridgeStatus;

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetBridgeStatusResponse(u8, u8);

impl Message for GetBridgeStatus {
    type Response = GetBridgeStatusResponse;

    fn serialize(&self) -> Vec<u8> {
        vec![0x05, 0x0]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetBridgeVersion;

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GetBridgeVersionResponse(u8, u8, u8);

impl Message for GetBridgeVersion {
    type Response = GetBridgeVersionResponse;

    fn serialize(&self) -> Vec<u8> {
        vec![0x03, 0x0]
    }
}
