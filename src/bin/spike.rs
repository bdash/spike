use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc, LazyLock},
    time::Duration,
};

use evdev::{
    uinput::{VirtualDevice, VirtualDeviceBuilder},
    AttributeSet, EventType, InputEvent, Key,
};
use log::{debug, info};

use spike::{NodeSwitchEvent, Result, Spike, SwitchEventKind};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Switch {
    LeftFlipperButton,
    LeftFlipperButtonUpper,
    RightFlipperButton,
    ActionButton,
    StartButton,
    Tilt,
    ServiceSelect,
    ServicePlus,
    ServiceMinus,
    ServiceBack,
    LeftCoin,
    CenterCoin,
    RightCoin,
    #[allow(dead_code)]
    Unknown(u8, u8),
}

impl Switch {
    fn key(&self) -> Option<Key> {
        Some(match self {
            Switch::LeftFlipperButton => Key::KEY_LEFT,
            Switch::RightFlipperButton => Key::KEY_RIGHT,
            Switch::ActionButton => Key::KEY_LEFTCTRL,
            Switch::StartButton => Key::KEY_UP,
            Switch::Tilt => Key::KEY_ESC,

            Switch::ServiceSelect => Key::KEY_ENTER,
            Switch::ServicePlus => Key::KEY_DOWN,
            Switch::ServiceMinus => Key::KEY_UP,
            Switch::ServiceBack => Key::KEY_ESC,

            Switch::LeftCoin => Key::KEY_2,
            Switch::CenterCoin => Key::KEY_3,
            Switch::RightCoin => Key::KEY_4,

            Switch::LeftFlipperButtonUpper => return None,
            Switch::Unknown(..) => return None,
        })
    }

    fn normally_open(&self) -> bool {
        match self {
            _ => false,
        }
    }

    fn is_toggle(&self) -> bool {
        match self {
            Switch::StartButton => true,
            _ => false,
        }
    }
}

impl From<(u8, u8)> for Switch {
    fn from(value: (u8, u8)) -> Self {
        *AIQ_PREMIUM_SWITCH_MAPPING
            .get(&value)
            .unwrap_or(&Switch::Unknown(value.0, value.1))
    }
}

trait SwitchEventConversion {
    fn value(&self) -> i32;
    fn inverted(&self) -> Self;
}

impl SwitchEventConversion for SwitchEventKind {
    fn value(&self) -> i32 {
        match self {
            SwitchEventKind::Close => 1,
            SwitchEventKind::Open => 0,
        }
    }

    fn inverted(&self) -> Self {
        match self {
            SwitchEventKind::Close => SwitchEventKind::Open,
            SwitchEventKind::Open => SwitchEventKind::Close,
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct SwitchEvent {
    kind: SwitchEventKind,
    switch: Switch,
}

impl TryFrom<SwitchEvent> for InputEvent {
    fn try_from(event: SwitchEvent) -> std::result::Result<Self, ()> {
        if let Some(key) = event.switch.key() {
            Ok(InputEvent::new(
                EventType::KEY,
                key.code(),
                event.kind.value(),
            ))
        } else {
            Err(())
        }
    }

    type Error = ();
}

impl From<NodeSwitchEvent> for SwitchEvent {
    fn from(switch_event: NodeSwitchEvent) -> Self {
        let switch = Switch::from((switch_event.node, switch_event.switch));
        let kind = if switch.normally_open() {
            switch_event.kind.inverted()
        } else {
            switch_event.kind
        };
        SwitchEvent { switch, kind }
    }
}

// Mapping taken from the manual.
static AIQ_PREMIUM_SWITCH_MAPPING: LazyLock<HashMap<(u8, u8), Switch>> = LazyLock::new(|| {
    HashMap::from_iter([
        ((8, 24), Switch::RightFlipperButton),
        ((8, 25), Switch::LeftFlipperButton),
        ((8, 27), Switch::LeftFlipperButtonUpper),
        ((1, 2), Switch::ActionButton),
        ((1, 11), Switch::StartButton),
        ((1, 14), Switch::Tilt),
        ((1, 16), Switch::LeftCoin),
        ((1, 17), Switch::RightCoin),
        ((1, 18), Switch::CenterCoin),
        ((0, 8), Switch::ServiceSelect),
        ((0, 9), Switch::ServicePlus),
        ((0, 10), Switch::ServiceMinus),
        ((0, 11), Switch::ServiceBack),
    ])
});

fn dispatch_events(events: Vec<SwitchEvent>, device: &mut VirtualDevice) -> Result<()> {
    let input_events: Vec<_> = events
        .into_iter()
        .filter_map(|e| {
            e.try_into()
                .map_err(|_| debug!("Ignoring event without keyboard mapping: {:?}", e))
                .ok()
        })
        .collect();
    if input_events.len() > 0 {
        info!("Emitting keyboard events: {:?}", input_events);
        device.emit(&input_events)?;
    }
    Ok(())
}

struct StatefulEvents {
    state: HashMap<Switch, SwitchEventKind>,
}

impl StatefulEvents {
    fn new() -> StatefulEvents {
        StatefulEvents {
            state: HashMap::new(),
        }
    }

    fn translate(&mut self, events: Vec<SwitchEvent>) -> Vec<SwitchEvent> {
        events
            .into_iter()
            .filter_map(|event| match (event.switch.is_toggle(), event.kind) {
                // Normal, non-toggle switch. Pass event through.
                (false, _) => Some(event),

                // Toggle switch. Ignore close.
                (true, SwitchEventKind::Close) => None,

                // Toggle switch. On open, toggle the state and report the corresponding event.
                (true, SwitchEventKind::Open) => {
                    let kind = self
                        .state
                        .entry(event.switch)
                        .and_modify(|k| *k = k.inverted())
                        .or_insert(SwitchEventKind::Close);
                    Some(SwitchEvent {
                        switch: event.switch,
                        kind: *kind,
                    })
                }
            })
            .collect()
    }
}

fn main() -> Result<()> {
    stderrlog::new()
        .module(module_path!())
        .verbosity(log::Level::Debug)
        .init()
        .unwrap();

    let keys = AttributeSet::<Key>::from_iter(&[
        Key::KEY_LEFT,
        Key::KEY_RIGHT,
        Key::KEY_UP,
        Key::KEY_DOWN,
        Key::KEY_SPACE,
        Key::KEY_ENTER,
        Key::KEY_LEFTCTRL,
        Key::KEY_ESC,
        Key::KEY_0,
        Key::KEY_1,
        Key::KEY_2,
        Key::KEY_3,
        Key::KEY_4,
    ]);

    let mut device = VirtualDeviceBuilder::new()?
        .name("Spike 2 pinball input")
        .with_keys(&keys)?
        .build()
        .unwrap();

    let should_exit = Arc::new(AtomicBool::new(false));

    {
        let should_exit = should_exit.clone();
        ctrlc::set_handler(move || {
            should_exit.store(true, std::sync::atomic::Ordering::SeqCst);
        })?;
    }

    let mut spike = Spike::new()?;
    info!("Initializing Spike system");
    spike.initalize()?;

    let mut stateful_events = StatefulEvents::new();
    while !should_exit.load(std::sync::atomic::Ordering::SeqCst) {
        let events: Vec<_> = spike
            .wait_for_switch_event(Duration::from_millis(50), should_exit.clone())?
            .into_iter()
            .map(|e| e.into())
            .collect();
        let events = stateful_events.translate(events);
        dispatch_events(events, &mut device)?
    }

    info!("Quitting");

    Ok(())
}
