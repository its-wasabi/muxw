mod worker;
mod xkb_manager;

pub struct InputManager {
    // TODO: Kept for possibility of future runtime configuration of worker thread
    // If its truly unnecessary remove it
    #[allow(unused)]
    worker_sender: crate::event_loop::EventSender<worker::InputToken>,
}

impl InputManager {
    pub fn new(
        sender: crate::event_loop::EventSender<crate::token::Token>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        println!("INPUT - START");
        let (worker_sender, worker_resources) = worker::InputWorker::new(sender)?;

        println!("INPUT (THREAD)");
        std::thread::Builder::new()
            .name(std::string::String::from("input-worker"))
            .spawn(move || worker::run_input_worker_thread(worker_resources))?;

        println!("INPUT - DONE");

        Ok(Self { worker_sender })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputEvent {
    key: InputDeviceKey,
    kind: InputEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InputDeviceKey(usize);

impl InputDeviceKey {
    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEventKind {
    DeviceAdded,
    DeviceRemoved,
    Keyboard {
        keycode: u32,
        state: input::event::keyboard::KeyState,
    },
    PointerButton {
        keycode: u32,
        state: input::event::pointer::ButtonState,
    },

    PointerVerticalScroll {
        scroll: f64,
    },

    PointerHorizontalScroll {
        scroll: f64,
    },

    Motion {
        delta_x: f64,
        delta_y: f64,
    },
}
