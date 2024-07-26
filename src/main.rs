extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use crossbeam_channel::{Receiver, Sender};
use nwg::{HTextAlign, NativeUi, VTextAlign};
use serialport::available_ports;

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

pub struct FileMenu {
    file_menu: nwg::Menu,
    quit_butto: nwg::MenuItem,
}

pub struct BasicAppState {
    window: nwg::Window,
    result: nwg::TextInput,
    spawn_button: nwg::Button,
    text_box_font: nwg::Font,
    ports_combo_list: nwg::ComboBox<String>,
    ports_combo_label: nwg::Label,
    notice: nwg::Notice,
    channel: RefCell<(Sender<u32>, Receiver<u32>)>,
    logs: nwg::RichTextBox,
    file_menu: FileMenu,
}

impl BasicAppState {
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
            .flags(
                nwg::WindowFlags::MAIN_WINDOW | nwg::WindowFlags::VISIBLE, //| nwg::WindowFlags::SYS_MENU,
            )
            .size((680, 460))
            .position((300, 300))
            .title("Winserial")
            .build(&mut data.window)?;

        nwg::Label::builder()
            .position((10, 10))
            .text("Port:")
            .v_align(VTextAlign::Center)
            .parent(&data.window)
            .build(&mut data.ports_combo_label)?;

        let mut col = Vec::new();
        col.push("First".to_string());
        col.push("Second".to_string());

        nwg::ComboBox::builder()
            .position((150, 10))
            .parent(&data.window)
            .collection(col)
            .build(&mut data.ports_combo_list)?;

        nwg::TextInput::builder()
            .size((280, 25))
            .position((10, 40))
            .text("Result")
            .parent(&data.window)
            .focus(false)
            .readonly(true)
            .build(&mut data.result)?;
        nwg::RichTextBox::builder()
            //.flags(RichTextBoxFlags::AUTOVSCROLL)
            .size((280, 100))
            .parent(&data.window)
            .readonly(true)
            .font(Some(&data.text_box_font))
            .position((10, 160))
            .build(&mut data.logs)?;

        nwg::Font::builder()
            .family("Courier New")
            .size(12)
            .build(&mut data.text_box_font)?;

        nwg::Button::builder()
            .position((10, 70))
            .text("Spawn")
            .parent(&data.window)
            .build(&mut data.spawn_button)?;

        nwg::Notice::builder()
            .parent(&data.window)
            .build(&mut data.notice)?;

        nwg::Menu::builder()
            .parent(&data.window)
            .popup(false)
            .text("File")
            .build(&mut data.file_menu.file_menu)?;

        nwg::MenuItem::builder()
            .text("Quit")
            .parent(&data.file_menu.file_menu)
            .build(&mut data.file_menu.quit_butto)?;

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
                    E::OnMenuItemSelected => {
                        if &handle == &ui.file_menu.quit_butto {
                            nwg::stop_thread_dispatch();
                        }
                    }
                    E::OnComboxBoxSelection => {
                        if &handle == &ui.ports_combo_list {
                            if let Some(string) = ui.ports_combo_list.selection_string() {
                                ui.result.set_text(string.as_str());
                            }
                        }
                    }
                    E::OnComboBoxDropdown => {
                        if &handle == &ui.ports_combo_list {
                            match available_ports() {
                                Ok(ports) => {
                                    let collection =
                                        ports.iter().map(|port| port.port_name.clone()).collect();
                                    ui.ports_combo_list.set_collection(collection);
                                }
                                _ => {}
                            }
                        }
                    }
                    E::OnButtonClick => {
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
                            let mut logs = ui.logs.text();
                            logs.push_str(format!("Count is {}\r\n", recv).as_str());
                            ui.logs.set_text(logs.as_str());
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

    nwg::Font::set_global_family("Calibri").expect("Failed to set default font");

    let (send, recv): (Sender<u32>, Receiver<u32>) = crossbeam_channel::unbounded();
    let channel = RefCell::new((send, recv));
    let file_menu = FileMenu {
        file_menu: nwg::Menu::default(),
        quit_butto: nwg::MenuItem::default(),
    };
    let app_state = BasicAppState {
        window: nwg::Window::default(),
        result: nwg::TextInput::default(),
        spawn_button: nwg::Button::default(),
        notice: nwg::Notice::default(),
        text_box_font: nwg::Font::default(),
        ports_combo_label: nwg::Label::default(),
        ports_combo_list: nwg::ComboBox::default(),
        channel,
        logs: nwg::RichTextBox::default(),
        file_menu,
    };

    let _ui = BasicAppState::build_ui(app_state).expect("Error.");
    nwg::dispatch_thread_events();
}
