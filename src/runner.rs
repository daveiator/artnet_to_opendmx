use crate::cli::Arguments;

use std::{fmt::{Display, Formatter}, net::SocketAddr, sync::mpsc};

use artnet_protocol::{PortAddress, PollReply};
use artnet_reciever::ArtnetRecieverBuilder;
use open_dmx::DMXSerial;
use serialport::available_ports;
use log::{info, debug, warn, error};


pub type RunnerUpdateReciever = mpsc::Receiver<RunnerUpdate>;

#[derive(Default, Debug, Clone, Copy)] //all false
pub struct RunnerUpdate {
    pub dmx_recieved: Option<SocketAddr>,
    pub dmx_sent: bool,
    
    pub connected_to_artnet: bool,
    pub connected_to_dmx: bool,
}

pub fn create_runner(arguments: Arguments) -> Result<RunnerUpdateReciever, RunnerCreationError> {
    let (tx, rx) = mpsc::sync_channel(1);
    
    info!("Checking for device named \"{}\"...", arguments.device_name);
    let ports= match available_ports() {
        Err(error) => {
            error!("Coulnd't get available ports list: {}", error);
            return Err(RunnerCreationError::PortListingError(error));
        },
        Ok(ports) => ports,
    };
    let mut device = "";
    for port in ports {
        if port.port_name == arguments.device_name {
            info!("Found device \"{}\"", arguments.device_name);
            device = &arguments.device_name;
            break;
        }
    }
    if device.is_empty() {
        error!("Couldn't find device named \"{}\"", arguments.device_name);
        return Err(RunnerCreationError::LocateDeviceError);
    }
    info!("Starting dmx interface...");
    let mut dmx = match DMXSerial::open_sync(device) {
        Ok(dmx) => dmx,
        Err(error) => {
            error!("Couldn't open dmx interface: {}", error);
            return Err(RunnerCreationError::DeviceOpeningError(error));
        },
    };
    if let Some(time) = arguments.options.break_time {
        debug!("Setting dmx interface break time to {}ms", time.as_millis());
        dmx.set_packet_time(time);
    }
    if arguments.options.remember {
        debug!("Setting dmx interface to remember mode");
        dmx.set_async();
        dmx.set_channels([0; 512]);
        if let Err(error) = dmx.update_async() {
            return Err(RunnerCreationError::DeviceUpdateError(error));
        }
    }
    info!("Started!");


    info!("Starting art-net listener...");

    debug!("Creating art-net poll reply packet...");
    let output = match arguments.options.controller.is_some() {
        true => 0x8A,
        false => 0x80,
    };
    let mut short_name = [0; 18];
    "artnet2opendmx".bytes().enumerate().for_each(|(i, b)| short_name[i] = b);
    let mut long_name = [0; 64];
    match &arguments.options.name {
        Some(name) if name.as_bytes().len() <= 64 => name.clone(),
        _ => "artnet_to_opendmx_node".into(),
    }.as_bytes().iter().zip(long_name.iter_mut()).for_each(|(a, b)| *b = *a);

    let poll_reply = PollReply {
        address: [0, 0, 0, 0].into(),
        port: 0,
        version: [1, 0],
        port_address: arguments.universe.to_be_bytes(),
        oem: [0; 2],
        ubea_version: 0,
        status_1: 0,
        esta_code: 0,
        short_name,
        long_name,
        node_report: [0; 64],
        num_ports: [0, 1],
        port_types: [0x40, 0, 0, 0],
        good_input: [8; 4],
        good_output: [output, 0, 0, 0],
        swin: [0; 4],
        swout: [0; 4],
        sw_video: 0,
        sw_macro: 0,
        sw_remote: 0,
        style: 0x00,
        mac: [0; 6],
        bind_ip: [0, 0, 0, 0].into(),
        bind_index: 1,
        status_2: 0,
        filler: [0; 26],
        spare: [0; 3],
    };

    debug!("Creating art-net reciever...");

    let artnet_reciever_template = ArtnetRecieverBuilder::default()
        .socket_address(format!("{}:{}", arguments.options.controller.clone().unwrap_or("0.0.0.0".into()), arguments.options.port.unwrap_or(6454)).parse().unwrap()) //Port must be valid if the value is 16 bit
        .poll_reply(poll_reply);

    let artnet_output = match artnet_reciever_template.build() {
        Ok(reciever) => reciever,
        Err(error) => {
            error!("Couldn't create art-net reciever: {}", error);
            return Err(RunnerCreationError::ArtnetCreationError(error));
        },
    };

    info! ("Started!");
    std::thread::spawn(move || {
        let mut update = RunnerUpdate::default();
        loop {
            update.dmx_sent = false;
            update.dmx_recieved = None;

            if dmx.is_async() {
                update.dmx_sent = true;
            }

            match artnet_output.try_recv() {
                Ok((sender, output)) => {
                    update.connected_to_artnet = true;
                    if output.port_address == PortAddress::try_from(arguments.universe).unwrap() {
                        update.dmx_recieved = Some(sender);
                        debug!("Received output for universe {} from {}", arguments.universe, sender);
                        let mut channels = [0; 512];
                        output.to_bytes().unwrap()[8..].iter().zip(channels.iter_mut()).for_each(|(a, b)| *b = *a);
                        dmx.set_channels(channels);
                        update.dmx_sent = true;
                        match dmx.update() {
                            Ok(_) => {
                                update.dmx_sent = true;
                            },
                            Err(_) => {
                                error!("Couldn't update dmx channels. Interface got disconnected.");
                                debug!("Trying to reconnect...");
                                if let Err(e) = dmx.reopen() {
                                    error!("Couldn't reconnect to dmx interface: {}", e);
                                    update.dmx_sent = false;
                                }
                            },
                        }
                        debug!("Updated dmx channels on interface");
                    }
                },
                Err(mpsc::TryRecvError::Empty) => {
                    update.connected_to_artnet = true;
                    std::thread::sleep(std::time::Duration::from_millis(1));
                },
                Err(mpsc::TryRecvError::Disconnected) => {
                    error!("Art-net reciever disconnected");
                    std::thread::sleep(std::time::Duration::from_millis(1));
                },

            }
            update.connected_to_dmx = dmx.check_agent().is_ok();
            match tx.try_send(update) {
                Ok(_) => {},
                Err(mpsc::TrySendError::Full(_)) => {},
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    warn!("Update channel disconnected. Stopping runner...");
                    break;
                },
            }
        }
    });
    Ok(rx)
}

#[derive(Debug)]
pub enum RunnerCreationError {
    PortListingError(serialport::Error),
    LocateDeviceError,
    DeviceOpeningError(serialport::Error),
    DeviceUpdateError(open_dmx::error::DMXDisconnectionError),
    ArtnetCreationError(std::io::Error),
}

impl Display for RunnerCreationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RunnerCreationError::PortListingError(e) => write!(f, "Couldn't list serial ports: {}", e),
            RunnerCreationError::LocateDeviceError => write!(f, "Couldn't find device"),
            RunnerCreationError::DeviceOpeningError(e) => write!(f, "Couldn't open device: {}", e),
            RunnerCreationError::DeviceUpdateError(e) => write!(f, "Couldn't update device: {}", e),
            RunnerCreationError::ArtnetCreationError(e) => write!(f, "Couldn't create art-net reciever: {}", e),
        }
    }    
}