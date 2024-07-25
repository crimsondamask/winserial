extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use crossbeam_channel::{Receiver, Sender};
use nwd::NwgUi;
use nwg::NativeUi;

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

pub struct BasicAppState {
    window: nwg::Window,
    name_edit: nwg::TextInput,
    hello_button: nwg::Button,
    spawn_button: nwg::Button,
    ports_combo_list: nwg::ComboBox<&'static str>,
    notice: nwg::Notice,
    channel: RefCell<(Sender<u32>, Receiver<u32>)>,
}

impl BasicAppState {
    fn say_hello() {
        nwg::simple_message("Greetings", "Hello there!");
    }

    fn say_goodbye() {
        nwg::simple_message("Goodbyes", "Goodbye!");
        nwg::stop_thread_dispatch();
    }
}

pub struct BasicAppUi {
    inner: Rc<BasicAppState>,
    default_handler: RefCell<Option<nwg::EventHandler>>,
}

impl Drop for BasicAppUi {
    /// To make sure that everything is freed without issues, the default handler must be unbound.
    fn drop(&mut self) {
        let handler = self.default_handler.borrow();
        if handler.is_some() {
            nwg::unbind_event_handler(handler.as_ref().unwrap());
        }
    }
}

impl Deref for BasicAppUi {
    type Target = BasicAppState;

    fn deref(&self) -> &BasicAppState {
        &self.inner
    }
}

impl nwg::NativeUi<BasicAppUi> for BasicAppState {
    fn build_ui(mut data: BasicAppState) -> Result<BasicAppUi, nwg::NwgError> {
        use nwg::Event as E;

        // Controls
        nwg::Window::builder()
            .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
            .size((300, 115))
            .position((300, 300))
            .title("Basic example")
            .build(&mut data.window)?;

        nwg::TextInput::builder()
            .size((280, 25))
            .position((10, 10))
            .text("Heisenberg")
            .parent(&data.window)
            .focus(false)
            .readonly(true)
            .build(&mut data.name_edit)?;

        nwg::Button::builder()
            .position((10, 40))
            .text("Spawn")
            .parent(&data.window)
            .build(&mut data.spawn_button)?;

        nwg::Button::builder()
            .position((10, 40))
            .text("Say my name")
            .parent(&data.window)
            .build(&mut data.hello_button)?;

        nwg::ComboBox::builder()
            .position((10, 70))
            .parent(&data.window)
            .collection(vec!["First", "Second"])
            .selected_index(Some(0))
            .build(&mut data.ports_combo_list)?;

        // Wrap-up
        let ui = BasicAppUi {
            inner: Rc::new(data),
            default_handler: Default::default(),
        };

        // Events
        let evt_ui = Rc::downgrade(&ui.inner);
        let handle_events = move |evt, _evt_data, handle| {
            if let Some(ui) = evt_ui.upgrade() {
                match evt {
                    E::OnButtonClick => {
                        if &handle == &ui.hello_button {
                            //BasicAppState::say_hello();
                            ui.name_edit.set_text("Clicked!");
                        }

                        if &handle == &ui.spawn_button {
                            let send = ui.channel.borrow_mut().0.clone();
                            let sender = ui.notice.sender();
                            thread::spawn(move || {
                                let mut i = 0;
                                loop {
                                    i += 1;
                                    thread::sleep(Duration::from_secs(1));

                                    send.send(i).unwrap();
                                    sender.notice();
                                }
                            });
                        }
                    }
                    E::OnWindowClose => {
                        if &handle == &ui.window {
                            BasicAppState::say_goodbye();
                        }
                    }
                    E::OnNotice => {
                        if let Ok(recv) =
                            &ui.channel.borrow().1.recv_timeout(Duration::from_secs(2))
                        {
                            ui.name_edit
                                .set_text(format!("Count is {}", *recv).as_str());
                        }
                    }

                    _ => {}
                }
            }
        };

        *ui.default_handler.borrow_mut() = Some(nwg::full_bind_event_handler(
            &ui.window.handle,
            handle_events,
        ));

        return Ok(ui);
    }
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    let (send, recv): (Sender<u32>, Receiver<u32>) = crossbeam_channel::unbounded();
    let channel = RefCell::new((send, recv));
    let app_state = BasicAppState {
        window: nwg::Window::default(),
        name_edit: nwg::TextInput::default(),
        hello_button: nwg::Button::default(),
        spawn_button: nwg::Button::default(),
        notice: nwg::Notice::default(),
        ports_combo_list: nwg::ComboBox::default(),
        channel,
    };

    let _ui = BasicAppState::build_ui(app_state).expect("Error.");
    nwg::dispatch_thread_events();
}
