use std::net::SocketAddr;
use std::time::Instant;

use crate::cli::Arguments;
use crate::runner::{self, RunnerUpdateReciever, RunnerCreationError};

use eframe::egui;

use serialport::available_ports;



pub fn run_app(argument_option: Option<Arguments>) -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions{
        decorated: false,
        transparent: true,
        initial_window_size: Some(egui::Vec2::new(350.0, 200.0)),
        resizable: false,
        centered: true,
        ..Default::default()
    };
    eframe::run_native("artnet to opendmx", native_options, Box::new(|cc| Box::new(App::new(argument_option))))?;
    Ok(())
}

struct App {
    available_ports: Vec<serialport::SerialPortInfo>,
    runner: Option<RunnerUpdateReciever>,
    leds: Leds,
    last_packet_instant: Option<std::time::Instant>,
    last_packet: Option<(std::time::Duration, SocketAddr)>,
    current_settings: Option<Arguments>,
    new_settings: Option<Arguments>,
    settings_window_open: bool,

}

impl App {
    fn new(argument_option: Option<Arguments>) -> Self {
        App {
            available_ports: available_ports().unwrap(),
            runner: None,
            leds: Leds::default(),
            last_packet_instant: None,
            last_packet: None,
            current_settings: None,
            new_settings: argument_option,
            settings_window_open: false,
        }
    }

    fn start_runner(&mut self) -> Result<(), RunnerStartError> {
        self.runner = match runner::create_runner(match self.new_settings.as_ref() {
            Some(args) => args.clone(),
            None => return Err(RunnerStartError::NoConfig),
        }) {
            Ok(runner_update_reciever) => Some(runner_update_reciever),
            Err(error) => {
                return Err(RunnerStartError::RunnerCreationError(error));
                
            },
        };
        self.current_settings = self.new_settings.clone();
        self.last_packet_instant = Some(Instant::now());
        Ok(())
    }

    fn stop_runner(&mut self) {
        self.runner = None;
        self.leds = Leds::default();
        self.last_packet_instant = None;
        self.last_packet = None;
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

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut settings = false;
        custom_window_frame(ctx, frame, "artnet to opendmx", |ui| {
            ui.style_mut().animation_time = 20.0;

            if let Some(runner) = &self.runner {
                match runner.recv() {
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
                    Err(_) => {
                        self.stop_runner()
                    },
                }
            }

            
            
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
                    if self.runner.is_some() {
                        if ui.add(egui::Button::new("Stop").min_size(egui::vec2(50.0, 0.0))).clicked() {
                            self.stop_runner();
                        }
                    } else {
                        if ui.add_enabled(self.new_settings.is_some(), egui::Button::new("Start").min_size(egui::vec2(50.0, 0.0))).clicked() {
                            let _ = self.start_runner(); //TODO
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
            
        });
    }
}

fn custom_window_frame(
    ctx: &egui::Context,
    frame: &mut eframe::Frame,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    use egui::*;

    let panel_frame = egui::Frame {
        fill: ctx.style().visuals.window_fill(),
        rounding: 10.0.into(),
        stroke: egui::Stroke::NONE,
        ..Default::default()
    };

    CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
        let app_rect = ui.max_rect();
        if ui.interact(app_rect, Id::new("window"), Sense::click()).is_pointer_button_down_on() {
            frame.drag_window();
        }

        let title_bar_height = 32.0;
        let title_bar_rect = {
            let mut rect = app_rect;
            rect.max.y = rect.min.y + title_bar_height;
            rect
        };
        title_bar_ui(ui, frame, title_bar_rect, title);

        // Add the contents:
        let content_rect = {
            let mut rect = app_rect;
            rect.min.y = title_bar_rect.max.y;
            rect
        }
        .shrink(4.0);
        let mut content_ui = ui.child_ui(content_rect, *ui.layout());
        add_contents(&mut content_ui);
    });
}

fn title_bar_ui(
    ui: &mut egui::Ui,
    frame: &mut eframe::Frame,
    title_bar_rect: eframe::epaint::Rect,
    title: &str,
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
            if settings_response.clicked() {
            }
        });
    });

    //Window buttons
    ui.allocate_ui_at_rect(title_bar_rect, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.visuals_mut().button_frame = false;
            ui.add_space(8.0);
            close_maximize_minimize(ui, frame);
        });
    });
}

fn close_maximize_minimize(ui: &mut egui::Ui, frame: &mut eframe::Frame) {
    use egui::{Button, RichText};

    let button_height = 12.0;

    let close_response = ui.add(Button::new(RichText::new("❌").size(button_height)));
    if close_response.clicked() {
        frame.close();
    }

    let minimized_response = ui.add(Button::new(RichText::new("➖").size(button_height)));
    if minimized_response.clicked() {
        frame.set_minimized(true);
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

enum RunnerStartError {
    NoConfig,
    RunnerCreationError(RunnerCreationError),
}

#[derive(Default)]
struct Leds {
    link: bool,
    dmx: bool,
    com: bool,
    act: bool,
}