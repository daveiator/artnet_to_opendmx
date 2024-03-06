use std::net::{SocketAddr, Ipv4Addr};
use std::sync::mpsc::TryRecvError;
use std::time::Instant;

use crate::cli::Arguments;
use crate::runner::{self, RunnerUpdateReciever};

use crate::CARGO_PKG_VERSION;

use eframe::egui::{self, ViewportCommand};

use serialport::{available_ports, SerialPortType};

use log::{info, error};

const WINDOW_SIZE: egui::Vec2 = egui::Vec2::new(350.0, 200.0);
const SETTINGS_SIZE: egui::Vec2 = egui::Vec2::new(350.0, 300.0);

pub fn run_app(argument_option: Option<Arguments>) -> Result<(), Box<dyn std::error::Error>> {

    let native_options = eframe::NativeOptions{
        centered: true,
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_icon(load_icon())
            .with_inner_size([350.0, 200.0])
            .with_resizable(false)
            .with_transparent(true),
        ..Default::default()
    };
    eframe::run_native("artnet to opendmx", native_options, Box::new(|_| Box::new(App::new(argument_option))))?;
    Ok(())
}

pub(crate) fn load_icon() -> egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("../assets/embedded_icon.png");
        let image = image::load_from_memory(icon)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

struct App {
    available_ports: Vec<serialport::SerialPortInfo>,
    runner: Option<RunnerUpdateReciever>,
    leds: Leds,
    last_packet_instant: Option<std::time::Instant>,
    last_packet: Option<(std::time::Duration, SocketAddr)>,
    current_settings: Option<Arguments>,
    temp_config: Option<TempConfig>,
    settings_window_open: bool,
    manufacturer_filter: bool,
    gui_error_message: String,
    runner_waiting_for_restart: Option<std::time::Instant>,

}

impl App {
    fn new(argument_option: Option<Arguments>) -> Self {
        let mut app = App {
            available_ports: available_ports().unwrap(),
            runner: None,
            leds: Leds::default(),
            last_packet_instant: None,
            last_packet: None,
            current_settings: argument_option,
            temp_config: None,
            settings_window_open: false,
            manufacturer_filter: true,
            gui_error_message: String::new(),
            runner_waiting_for_restart: None,
        };

        if app.current_settings.is_some() {
            app.start_runner();
        }
        app
    }

    fn start_runner(&mut self) {
        self.runner = match runner::create_runner(match self.current_settings.as_ref() {
            Some(args) => args.clone(),
            None => {
                self.gui_error_message = "Error while starting: No config found".into();
                return;
            },
        }) {
            Ok(runner_update_reciever) => Some(runner_update_reciever),
            Err(error) => {
                self.gui_error_message = format!("Error while starting: {}", error);
                return;
            },
        };
        self.last_packet_instant = Some(Instant::now());
    }

    fn stop_runner(&mut self) {
        self.runner = None;
        self.leds = Leds::default();
        self.last_packet_instant = None;
        self.last_packet = None;
    }

    fn restart_runner(&mut self) {
        if self.runner.is_none() {
            return;
        }
        self.stop_runner();
        self.runner_waiting_for_restart = Some(Instant::now());

    }

    fn status_display(&self, ui: &mut egui::Ui, width: f32) {

        let bg_color = egui::Color32::from_rgb(0, 0, 0);
        let fg_color = egui::Color32::from_rgb(167, 219, 235);
        let fg_color = if self.runner.is_none() {
            fg_color.gamma_multiply(0.5)
        } else {
            fg_color
        };


        let size = egui::vec2(width, width/1.61803398875);
        let (_, mut rect) = ui.allocate_space(size);

        ui.painter().rect_filled(rect, width*0.02, fg_color);
        let org_rect = rect.clone();
        rect = rect.shrink(2.0);
        rect.set_top(rect.top() + 12.0);
        rect.set_height(40.0);
        let name_rect = rect.clone();
        ui.painter().rect_filled(rect, width*0.02, bg_color);
        rect.set_top(rect.bottom() + 2.0);
        rect.set_bottom(org_rect.bottom() - 2.0);
        ui.painter().rect_filled(rect, width*0.02, bg_color);

        ui.painter().text(
            org_rect.center_top(),
            egui::Align2::CENTER_TOP,
            "Device Info",
            egui::FontId::monospace(13.0),
            bg_color,
        );
        if self.runner.is_some() {
            ui.painter().text(
                name_rect.center(),
                egui::Align2::CENTER_CENTER,
                match self.current_settings.as_ref() {
                    Some(args) => format!("{}", match args.options.name.clone() {
                        Some(name) => name,
                        None => "artnet2opendmx".into()
                    }),
                    None => "No Config".to_string(),
                },
                egui::FontId::monospace(20.0),
                fg_color,
            );

            if let Some(arguments) = &self.current_settings {
                ui.painter().text(
                    rect.center_top(),
                    egui::Align2::CENTER_TOP,
                    format!("Listen: {}@{}", match &arguments.options.controller {
                        Some(controller) => controller,
                        None => "BROADCAST",
                    }, match arguments.options.port {
                        Some(port) => port,
                        None => 6454,
                    }),
                    egui::FontId::monospace(10.0),
                    fg_color,
                );
                rect.set_top(rect.top() + 10.0);
                ui.painter().text(
                    rect.center_top(),
                    egui::Align2::CENTER_TOP,
                    format!("Universe: {} ⏵ COM: {}", arguments.universe, arguments.device_name),
                    egui::FontId::monospace(10.0),
                    fg_color,
                );
                rect.set_top(rect.top() + 10.0);
                ui.painter().text(
                    rect.center_top(),
                    egui::Align2::CENTER_TOP,
                    format!("Remembering: {}", if arguments.options.remember { "True" } else { "False"}),
                    egui::FontId::monospace(10.0),
                    fg_color,
                );

                // Debugs Info
                if let Some((last_packet, sender)) = &self.last_packet {
                    ui.painter().text(
                        rect.center_bottom(),
                        egui::Align2::CENTER_BOTTOM,
                        format!("Sender: {}", sender),
                        egui::FontId::monospace(10.0),
                        fg_color,
                    );
                    rect.set_bottom(rect.bottom() - 10.0);
                    ui.painter().text(
                        rect.center_bottom(),
                        egui::Align2::CENTER_BOTTOM,
                        format!("Last Packet: {:?}", last_packet),
                        egui::FontId::monospace(10.0),
                        fg_color,
                    );
                }

            }
        } else {
            ui.painter().text(
                name_rect.center(),
                egui::Align2::CENTER_CENTER,
                "O F F L I N E",
                egui::FontId::monospace(20.0),
                fg_color,
            );
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let panel_frame = egui::Frame {
            fill: ctx.style().visuals.window_fill(),
            rounding: 10.0.into(),
            stroke: egui::Stroke::NONE,
            ..Default::default()
        };
        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {

            ui.style_mut().interaction.selectable_labels = false;
            
            let app_rect = ui.max_rect();
            if ui.interact(app_rect, egui::Id::new("window"), egui::Sense::click()).is_pointer_button_down_on() {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }

            let title_bar_height = 32.0;
            let title_bar_rect = {
                let mut rect = app_rect;
                rect.max.y = rect.min.y + title_bar_height;
                rect
            };
            let mut settings_window = false;
            let title = if self.settings_window_open {
                "Settings"
            } else {
                "artnet2opendmx"
            };
            title_bar_ui(ui, title_bar_rect, title, &mut settings_window);
            if settings_window {
                ui.ctx().send_viewport_cmd(ViewportCommand::InnerSize(SETTINGS_SIZE));
                self.settings_window_open = true;
            }

            // Add the contents:
            let content_rect = {
                let mut rect = app_rect;
                rect.min.y = title_bar_rect.max.y;
                rect
            }
                .shrink(4.0);
            let mut ui = ui.child_ui(content_rect, *ui.layout());

            //LOGIC
            if let Some(instant) = &self.runner_waiting_for_restart {
                if instant.elapsed() > std::time::Duration::from_secs(1) {
                    self.runner_waiting_for_restart = None;
                    self.start_runner();
                }
            }
            if let Some(runner) = &self.runner {
                match runner.try_recv() {
                    Ok(update) => {
                        self.leds.link = update.connected_to_artnet;
                        self.leds.dmx = update.dmx_recieved.is_some();
                        self.leds.com = update.connected_to_dmx;
                        self.leds.act = update.dmx_sent;

                        if let Some(sender) = update.dmx_recieved {
                            self.last_packet = Some((self.last_packet_instant.unwrap().elapsed(), sender));
                            self.last_packet_instant = Some(Instant::now());
                        }
                        ctx.request_repaint();
                    },
                    Err(TryRecvError::Empty) => {
                        ctx.request_repaint();
                    },
                    Err(_) => {
                        self.stop_runner()
                    },
                }
            }
            //SETTINGS
            if self.settings_window_open {
                

                if self.temp_config.is_none() {
                    self.gui_error_message = "".into();
                    self.temp_config = Some(TempConfig::from(self.current_settings.clone().unwrap_or_default()));
                }

                let mut temp_config = self.temp_config.clone().unwrap();

                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.columns(2, |cols| {
                        cols[0].with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            ui.label(egui::RichText::new("Art-Net").heading().strong());
                            ui.separator();

                            ui.label(egui::RichText::new("Node Name:").underline().strong()).on_hover_text("max. 18 Characters");
                            ui.add(egui::TextEdit::singleline(&mut temp_config.artnet_name).desired_width(150.0));
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("Controller IP Address:").underline().strong());
                            ui.checkbox(&mut temp_config.broadcast,"Recieve Broadcast");
                            ui.add(egui::TextEdit::singleline(&mut temp_config.controller).desired_width(100.0).interactive(!temp_config.broadcast));
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("Port:").underline().strong()).on_hover_text("0-65535");
                            ui.add(egui::TextEdit::singleline(&mut temp_config.port).desired_width(50.0));
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("Universe:").underline().strong()).on_hover_text("0-32787");
                            ui.add(egui::TextEdit::singleline(&mut temp_config.universe).desired_width(50.0));

                        });
                        cols[1].with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            ui.label(egui::RichText::new("Open-DMX").heading().strong());
                            ui.separator();

                            ui.label(egui::RichText::new("Serial-Port:").underline().strong());
                            ui.horizontal(|ui| {
                                ui.style_mut().spacing.item_spacing.x = 0.0;
                                if ui.add(egui::Button::new(egui::RichText::new("🔄"))).clicked() {
                                    info!("Refreshing Serial Port List...");
                                    self.available_ports = serialport::available_ports().unwrap_or_default();
                                }
                                egui::ComboBox::from_id_source("serial_port_selection").selected_text(temp_config.serial_name.clone()).width(ui.available_width()-ui.available_height()).show_ui(ui, |ui| {
                                    for port in self.available_ports.iter() {
                                        let manufacturer = match &port.port_type {
                                            SerialPortType::UsbPort(info) => info.manufacturer.clone().unwrap_or("".into()),
                                            _ => "".into(),
                                        };
                                        if self.manufacturer_filter && !manufacturer.to_lowercase().contains("ftdi") {
                                            continue;
                                        }
                                        let port = format!("{}", port.port_name);
                                        // let port = port.port_name.clone();
                                        ui.selectable_value(&mut temp_config.serial_name, port.clone(), port);
                                    }
                                });
                            });
                            ui.checkbox(&mut self.manufacturer_filter, "Only show FTDI Devices");
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("Output:").underline().strong());
                            ui.checkbox(&mut temp_config.custom_break_time, "Custom Break Time");
                            if temp_config.custom_break_time {
                                ui.horizontal(|ui| {
                                    ui.add(egui::TextEdit::singleline(&mut temp_config.break_time).desired_width(20.0));
                                    ui.label(egui::RichText::new("ms"));
                                });
                            }
                            ui.checkbox(&mut temp_config.remember,"Remember last values");
                        });
                    })
                });
                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.add_space(2.0);
                        let cancel_button = ui.add(egui::Button::new(egui::RichText::new("Cancel")));
                        let apply_button = ui.add(egui::Button::new(egui::RichText::new("Apply")));
                        ui.label(egui::RichText::new(&self.gui_error_message).color(egui::Color32::RED));
                        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new(format!("v{}", CARGO_PKG_VERSION)).small());
                            });
                        });

                        self.temp_config = Some(temp_config.clone());

                        if cancel_button.clicked() {
                            self.settings_window_open = false;
                            self.temp_config = None;
                            ui.ctx().send_viewport_cmd(ViewportCommand::InnerSize(WINDOW_SIZE));
                            ui.ctx().send_viewport_cmd(ViewportCommand::Title(String::from("artnet to opendmx")));
                            self.gui_error_message = "".into();
                        }
                        if apply_button.clicked() {
                            let new_settings: Arguments = match TryInto::try_into(temp_config.clone()) {
                                Ok(arguments) => arguments,
                                Err(e) => {
                                    error!("Error while applying settings: {}", e);
                                    self.gui_error_message = format!("Error while applying settings: {}", e);
                                    return;
                                }
                            };
                            // self.current_settings = Some(temp_config.clone());
                            self.settings_window_open = false;
                            self.temp_config = None;
                            ui.ctx().send_viewport_cmd(ViewportCommand::InnerSize(WINDOW_SIZE));
                            ui.ctx().send_viewport_cmd(ViewportCommand::Title(String::from("artnet to opendmx")));
                            self.gui_error_message = "".into();

                            self.current_settings = Some(new_settings.clone());
                            self.restart_runner();
                        }
                    });
                });
                return;
            }

            //UI
            ui.add_space(10.0);
            ui.columns(3, |cols| {
                cols[0].with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(75.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        if self.runner.is_some() {
                            ui.style_mut().visuals.override_text_color = Some(ui.style().visuals.widgets.open.fg_stroke.color);
                        }
                        ui.add_space(10.0);
                        signal_lamp(ui, 12.0, egui::Color32::from_rgb(117, 255, 157), self.leds.link);
                        ui.label(egui::RichText::new("LINK").font(egui::FontId::proportional(15.0)).heading());
                        ui.add_space(45.0);
                        signal_lamp(ui, 12.0, egui::Color32::from_rgb(138, 199, 255), self.leds.dmx);
                        ui.label(egui::RichText::new("DMX").font(egui::FontId::proportional(15.0)).heading());
                    });
                });
                cols[1].with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    self.status_display(ui, 200.0);
                    ui.add_space(4.0);
                    if self.runner_waiting_for_restart.is_some() {
                        ui.add_enabled(false, egui::Button::new("Starting...").min_size(egui::vec2(50.0, 0.0)));
                    } else if self.runner.is_some() {
                        if ui.add(egui::Button::new("Stop").min_size(egui::vec2(50.0, 0.0))).clicked() {
                            self.stop_runner();
                        }
                    } else {
                        if ui.add(egui::Button::new("Start").min_size(egui::vec2(50.0, 0.0))).clicked() {
                            self.start_runner();
                        }
                    }
                });
                cols[2].with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                    ui.set_width(75.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        if self.runner.is_some() {
                            ui.style_mut().visuals.override_text_color = Some(ui.style().visuals.widgets.open.fg_stroke.color);
                        }
                        ui.add_space(10.0);
                        signal_lamp(ui, 12.0, egui::Color32::from_rgb(255, 96, 79), self.leds.com);
                        ui.label(egui::RichText::new("COM").font(egui::FontId::proportional(15.0)).heading());
                        ui.add_space(45.0);
                        signal_lamp(ui, 12.0, egui::Color32::from_rgb(255, 204, 102), self.leds.act);
                        ui.label(egui::RichText::new("ACT").font(egui::FontId::proportional(15.0)).heading());
                    });
                });
            });
            // Error overlay
            let mut window = egui::Rect { min: egui::pos2(0.0, 0.0), max: WINDOW_SIZE.to_pos2() };
            window.set_bottom(65.0);
            ui.put(window, egui::Label::new(egui::RichText::new(self.gui_error_message.clone()).color(egui::Color32::RED)));
        });
    }
}

fn title_bar_ui(
    ui: &mut egui::Ui,
    title_bar_rect: eframe::epaint::Rect,
    title: &str,
    settings_open: &mut bool,
) {
    use egui::*;

    let painter = ui.painter();

    // Paint the title:
    painter.text(
        title_bar_rect.center(),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(13.0),
        ui.style().visuals.text_color(),
    );

    // Settings button:
    ui.allocate_ui_at_rect(title_bar_rect, |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.visuals_mut().button_frame = false;
            ui.add_space(8.0);
            let settings_response = ui.add(Button::new(RichText::new("⛭").size(12.0)));
            *settings_open = settings_response.clicked();
        });
    });

    //Window buttons
    ui.allocate_ui_at_rect(title_bar_rect, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.visuals_mut().button_frame = false;
            ui.add_space(8.0);
            close_maximize_minimize(ui);
        });
    });
}

fn close_maximize_minimize(ui: &mut egui::Ui) {
    use egui::{Button, RichText};

    let button_height = 12.0;

    let close_response = ui.add(Button::new(RichText::new("❌").size(button_height)));
    if close_response.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }

    let minimized_response = ui.add(Button::new(RichText::new("➖").size(button_height)));
    if minimized_response.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

fn signal_lamp(ui: &mut egui::Ui, size: f32, color: egui::Color32, on: bool) {
    let (_, rect) = ui.allocate_space(egui::vec2(size, size));
    let color = if !on {
        color.gamma_multiply(0.2)
    } else {
        color
    };
    ui.painter().circle_filled(rect.center(), size/2.0, color);
}

#[derive(Default)]
struct Leds {
    link: bool,
    dmx: bool,
    com: bool,
    act: bool,
}

#[derive(Clone)]
struct TempConfig {
    broadcast: bool,
    controller: String,
    port: String,
    universe: String,
    artnet_name: String,
    serial_name: String,
    custom_break_time: bool,
    break_time: String,
    remember: bool,
    
}

impl Default for TempConfig {
    fn default() -> Self {
        Self {
            broadcast: true,
            controller: "0.0.0.0".into(),
            port: "6454".into(),
            universe: "0".into(),
            artnet_name: "artnet2opendmx".into(),
            serial_name: "".into(),
            custom_break_time: false,
            break_time: "".into(),
            remember: false,
        }
    }
}

impl From<Arguments> for TempConfig {
    fn from(args: Arguments) -> Self {
        let mut config = Self::default();
        if let Some(controller) = args.options.controller {
            config.controller = controller;
            config.broadcast = false;
        }
        if let Some(port) = args.options.port {
            config.port = port.to_string();
        }
        config.universe = args.universe.to_string();
        if let Some(artnet_name) = args.options.name {
            config.artnet_name = artnet_name;
        }
        config.serial_name = args.device_name;
        config.custom_break_time = args.options.break_time.is_some();
        config.break_time = args.options.break_time.map(|time| time.as_millis().to_string()).unwrap_or("25".into());
        config.remember = args.options.remember;

        config
    }
}

impl TryInto<Arguments> for TempConfig {
    type Error = String;

    fn try_into(self) -> Result<Arguments, Self::Error> {
        let mut args = Arguments::default();
        args.universe = self.universe.parse().map_err(|_| "Invalid universe".to_string())?;
        if args.universe > 32787 {
            return Err("Universe too high".into());
        }
        if self.serial_name.is_empty() {
            return Err("No device selected".into());
        }
        args.device_name = self.serial_name;
        if self.broadcast {
            args.options.controller = None;
        } else {
            let octets = self.controller.split('.').map(|s| s.parse::<u8>()).collect::<Result<Vec<_>, _>>().map_err(|_| "Invalid IP")?;
            let ip = Ipv4Addr::try_from([octets[0], octets[1], octets[2], octets[3]]).map_err(|_| "Invalid IP")?;
            args.options.controller = Some(ip.to_string());
        }
        args.options.port = Some(self.port.parse().map_err(|_| "Invalid port".to_string())?);

        if self.artnet_name.bytes().len() > 18 {
            return Err("Name too long".into());
        }
        args.options.name = Some(self.artnet_name);
        if self.custom_break_time {
            args.options.break_time = Some(std::time::Duration::from_millis(self.break_time.parse().map_err(|_| "Invalid break time".to_string())?));
        } else {
            args.options.break_time = None;
        }
        args.options.remember = self.remember;

        Ok(args)
    }
}
