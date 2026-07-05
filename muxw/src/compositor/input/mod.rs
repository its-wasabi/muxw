mod worker;

pub struct InputManager {
    xkb_state: xkbcommon::xkb::State,
    keymap: xkbcommon::xkb::Keymap,
}

impl InputManager {
    pub fn new(
        sender: crate::event_loop::EventSender<crate::Token>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let xkb_context = xkbcommon::xkb::Context::new(xkbcommon::xkb::CONTEXT_NO_FLAGS);
        let keymap = xkbcommon::xkb::Keymap::new_from_names(
            &xkb_context,
            "",
            "",
            "",
            "",
            None,
            xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .ok_or("failed to create xkb keymap")?;
        let xkb_state = xkbcommon::xkb::State::new(&keymap);

        let worker_resources = worker::WorkerResources::new()?;

        std::thread::Builder::new()
            .name(std::string::String::from("input-worker"))
            .spawn(move || worker::run_input_worker_thread(worker_resources, sender))?;

        Ok(Self { xkb_state, keymap })
    }

    pub fn process_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Device { .. } => {
                println!(
                    "\t\t\x1b[38;5;240m| \x1b[38;5;87mInput::POINTER::(>\x1b[38;5;198m Device Event (\x1b[38;5;196mNO DATA\x1b[38;5;198m)\x1b[0m",
                );
            }

            InputEvent::Keyboard { keycode, state } => {
                println!(
                    "\t\t\x1b[38;5;240m| \x1b[38;5;87mInput::KB::(>\x1b[38;5;198m {}\x1b[38;5;240m=\x1b[38;5;{}m{:?}\x1b[0m",
                    keycode,
                    if *state == input::event::keyboard::KeyState::Pressed {
                        118
                    } else {
                        196
                    },
                    state,
                );
            }

            InputEvent::Pointer { delta_x, delta_y } => {
                println!(
                    "\t\t\x1b[38;5;240m| \x1b[38;5;87mInput::POINTER::(>\x1b[38;5;198m {delta_x}\x1b[38;5;240m-\x1b[38;5;198m{delta_y}\x1b[0m",
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    Device {},

    Keyboard {
        keycode: u32,
        state: input::event::keyboard::KeyState,
    },

    Pointer {
        delta_x: f64,
        delta_y: f64,
    },
}
