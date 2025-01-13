use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Read,
    os::unix::fs::OpenOptionsExt,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use anyhow::Context as _;
use bitvec::{array::BitArray, order::Lsb0, slice::BitSlice, BitArr};
use gpio::GpioOut;
use messages::{bus::*, node::*, Message, Response};
use node_bus::NodeBus;

pub mod messages;
pub mod node_bus;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const SPI_DEVICE: &'static str = "/dev/spidev1.0";

#[derive(Debug, Copy, Clone)]
pub enum SwitchEventKind {
    Open,
    Close,
}

impl From<bool> for SwitchEventKind {
    fn from(value: bool) -> Self {
        if value {
            SwitchEventKind::Open
        } else {
            SwitchEventKind::Close
        }
    }
}

pub struct NodeSwitchEvent {
    pub node: u8,
    pub switch: u8,
    pub kind: SwitchEventKind,
}

pub struct Spike {
    node_bus: NodeBus,
    spi: File,
    switch_state: HashMap<u8, BitArr!(for 64, in u8)>,
}

impl Spike {
    pub fn new() -> Result<Spike> {
        let mut isp_pin = gpio::sysfs::SysFsGpioOutput::open(75).unwrap();
        isp_pin.set_high().context("Setting ISP failed.")?;

        let spi = OpenOptions::new()
            .write(true)
            .read(true)
            .custom_flags(libc::O_SYNC | libc::O_NOCTTY)
            .open(SPI_DEVICE)?;

        Ok(Spike {
            node_bus: NodeBus::new()?,
            spi,
            switch_state: HashMap::new(),
        })
    }

    pub fn initalize(&mut self) -> Result<()> {
        self.node_bus.send_message(GetBridgeState)?;
        self.node_bus.initialize()?;

        self.node_bus.send_message(GetBridgeState)?;

        self.node_bus
            .send_message(ToNode(0, SetTrafficFlags(0x22)))?;
        self.node_bus
            .send_message(ToNode(0, SetTrafficFlags(0x11)))?;

        let mut nodes = vec![];
        loop {
            let PollResponse(node) = self.node_bus.send_message(Poll)?;
            if node == 0 {
                break;
            }

            nodes.push(node);

            self.node_bus
                .send_message(ToNode(node, SetTrafficFlags(0x20)))?;
            self.node_bus
                .send_message(ToNode(node, SetTrafficFlags(0x10)))?;
        }
        nodes.push(0);

        self.node_bus
            .send_message(ToNode(0, SetTrafficFlags(0x22)))?;

        self.node_bus.send_message(GetBridgeVersion)?;

        for node in &nodes {
            self.node_bus.send_message(ToNode(*node, GetNodeVersion))?;
        }

        self.node_bus
            .send_message(ToNode(0, SetTrafficFlags(0x11)))?;

        for node in &nodes {
            self.node_bus.send_message(ToNode(*node, GetNodeStatus))?;
            self.node_bus
                .send_message(ToNode(*node, SetLEDsAndInputs(0x60, 0x40)))?;
            self.node_bus.send_message(ToNode(*node, GetNodeStatus))?;
        }

        // Read and ignore any pending switch inputs
        for node in &nodes {
            self.send_message(ToNode(*node, GetInputState))?;
        }

        // Populate the initial switch state
        for node in nodes {
            let node_state = self.send_message(ToNode(node, GetInputState))?;
            self.switch_state
                .insert(node, BitArray::new(node_state.0 .0));
        }
        let local_switch_state = self.read_local_switches()?;
        self.switch_state.insert(0, local_switch_state);

        Ok(())
    }

    pub fn send_message<M, R>(&mut self, msg: M) -> Result<R>
    where
        M: Message<Response = R>,
        R: Response,
    {
        self.node_bus.send_message(msg)
    }

    pub fn wait_for_switch_event(
        &mut self,
        timeout: Duration,
        should_exit: Arc<AtomicBool>,
    ) -> Result<Vec<NodeSwitchEvent>> {
        while !should_exit.load(std::sync::atomic::Ordering::SeqCst) {
            let PollResponse(node) = self.send_message(Poll)?;
            if node == 0 {
                let local_switches = self.read_local_switches()?;
                let switch_events = self.update_switch_state_for_node(0, &local_switches);
                if switch_events.len() > 0 {
                    return Ok(switch_events);
                }
                std::thread::sleep(timeout);
                continue;
            }

            let input_state = self.send_message(ToNode(node, GetInputState))?;
            let switch_events = self.update_switch_state_for_node(node, &input_state.0.switches());
            if switch_events.len() > 0 {
                return Ok(switch_events);
            }
        }

        Ok(vec![])
    }

    pub fn read_local_switches(&mut self) -> Result<BitArr!(for 64, in u8)> {
        let mut state = [0u8; 8];
        self.spi.read_exact(&mut state)?;
        Ok(state.into())
    }

    fn update_switch_state_for_node(
        &mut self,
        node: u8,
        new_state: &BitSlice<u8, Lsb0>,
    ) -> Vec<NodeSwitchEvent> {
        let previous_state = self.switch_state.get_mut(&node).unwrap();
        let changes = *previous_state ^ new_state;
        previous_state.copy_from_bitslice(new_state);

        let mut events = Vec::new();
        for switch in changes.iter_ones() {
            events.push(NodeSwitchEvent {
                node,
                switch: switch as u8,
                kind: new_state[switch].into(),
            })
        }
        events
    }
}
