use anyhow::Context as _;
use ioctl_rs as ioctl;
use log::trace;
use std::{
    ffi::c_int,
    io::{Read, Write as _},
    os::{fd::AsRawFd as _, unix::fs::OpenOptionsExt as _},
    time::Duration,
};
use termios::{cfmakeraw, cfsetspeed, os::linux::B460800, tcsetattr, TCSANOW};
use timeout_readwrite::TimeoutReader;

use crate::{messages::{Message, Response, bus::*, node::*}, Result};

const NODE_BUS_TTY_DEVICE: &'static str = "/dev/ttymxc1";
const NODE_BUS_BAUD_RATE: u32 = B460800;
const NODE_BUS_READ_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum NodeBusState {
    Open,
    Initialized,
}

pub struct NodeBus {
    fd: std::fs::File,
    reader: TimeoutReader<std::fs::File>,
    state: NodeBusState,
}

impl NodeBus {
    pub fn new() -> Result<NodeBus> {
        trace!("NodeBus::new");
        let fd = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .custom_flags(libc::O_SYNC | libc::O_NOCTTY)
            .open(NODE_BUS_TTY_DEVICE)
            .context("open")?;

        let mut termios = termios::Termios::from_fd(fd.as_raw_fd()).context("Termios::from_fd")?;
        cfmakeraw(&mut termios);
        cfsetspeed(&mut termios, NODE_BUS_BAUD_RATE).context("cfsetspeed")?;
        tcsetattr(fd.as_raw_fd(), TCSANOW, &termios).context("tcsetattr")?;

        // This might reset the netbridge CPU
        ioctl::tiocmbis(fd.as_raw_fd(), ioctl::TIOCM_RTS as c_int)
            .context("Setting RTS failed.")?;
        std::thread::sleep(Duration::from_millis(5 as u64));
        ioctl::tiocmbic(fd.as_raw_fd(), ioctl::TIOCM_RTS as c_int)
            .context("Clearing RTS failed.")?;
        std::thread::sleep(Duration::from_millis(5 as u64));

        let reader =
            TimeoutReader::new(fd.try_clone().context("try_clone")?, NODE_BUS_READ_TIMEOUT);
        Ok(NodeBus {
            fd,
            reader,
            state: NodeBusState::Open,
        })
    }

    pub fn initialize(&mut self) -> Result<()> {
        trace!("NodeBus::initialize");
        // Update our state immediately as if we fail during initialization we'll
        // want to attempt to tear down our state.
        self.state = NodeBusState::Initialized;

        self.send_message(SetPower(true))?;
        self.send_message(SetPower(false))?;
        std::thread::sleep(Duration::from_millis(500 as u64));

        self.send_message(SetAmpPower(true))?;
        std::thread::sleep(Duration::from_millis(500 as u64));

        self.send_message(SetPower(true))?;
        std::thread::sleep(Duration::from_millis(500 as u64));

        self.send_message(ToNode(0, Reset))?;
        std::thread::sleep(Duration::from_millis(250 as u64));

        Ok(())
    }

    #[allow(dead_code)]
    pub fn close(self) -> Result<()> {
        drop(self);

        Ok(())
    }

    pub fn send_message<M, R>(&mut self, msg: M) -> Result<R>
    where
        M: Message<Response = R>,
        R: Response,
    {
        let serialized = msg.serialize();
        trace!("NodeBus::send_message {:x?} {:02x?}", msg, serialized);
        self.fd.write(serialized.as_slice())?;
        if R::size() > 0 {
            let response_data = self.read_response(R::size())?;
            let response = R::from_bytes(&response_data)?;
            trace!("        response {:x?} {:02x?}", response, response_data);
            Ok(response)
        } else {
            Ok(R::from_bytes(&[])?)
        }
    }

    fn read_response(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0; size];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer)
    }
}

impl Drop for NodeBus {
    fn drop(&mut self) {
        if self.state == NodeBusState::Initialized {
            self.send_message(ToNode(0, Reset))
                .expect("Failed to send reset");
            self.send_message(SetPower(false))
                .expect("Failed to send power off");
        }
    }
}
