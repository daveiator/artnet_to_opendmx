mod cli;
use cli::*;

use serialport::{available_ports};
use open_dmx::DMXSerial;
use artnet_protocol::{PortAddress, PollReply};
use artnet_reciever::ArtnetRecieverBuilder;

use simple_logger::SimpleLogger;
use log::{info, debug, error};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = match Cli::parse() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("Couldn't parse arguments:\n{error}");
            eprintln!("Exiting...");
            std::process::exit(1);
        },
    };
    match args.command {
        Command::List => {
            println!("Available ports:");
            let ports = match available_ports() {
                Err(error) => {
                    eprintln!("Coulnd't get available ports list: {error}");
                    eprintln!("Exiting...");
                    std::process::exit(1);
                },
                Ok(ports) => ports,
            };
            for port in ports {
                if let serialport::SerialPortType::UsbPort(_) = port.port_type {
                    println!("  - \"{}\" (USB)", port.port_name);
                    continue;
                }
                println!("  - \"{}\"", port.port_name);
            }
            Ok(())
        },
        Command::Help =>  {
            println!("{HELP_TEXT}");
            Ok(())
        },
        Command::Version => {
            println!("artnet_to_opendmx 0.1.0");
            Ok(())
        },

        Command::Default(args) => {
            log_panics::init();
            SimpleLogger::new()
                .with_level(match args.options.verbose {
                    true => log::LevelFilter::Debug,
                    false => log::LevelFilter::Info,
                })
                .without_timestamps()
                .with_colors(true)
                .init()?;


            info!("Checking for device named \"{}\"...", args.device_name);
            let ports= match available_ports() {
                Err(error) => {
                    error!("Coulnd't get available ports list: {}", error);
                    eprintln!("Exiting...");
                    std::process::exit(1);
                },
                Ok(ports) => ports,
            };
            let mut device = "";
            for port in ports {
                if port.port_name == args.device_name {
                    info!("Found device \"{}\"", args.device_name);
                    device = &args.device_name;
                    break;
                }
            }
            if device.is_empty() {
                error!("Couldn't find device named \"{}\"", args.device_name);
                eprintln!("Exiting...");
                std::process::exit(1);
            }
            info!("Starting dmx interface...");
            let mut dmx = match DMXSerial::open_sync(device) {
                Ok(dmx) => dmx,
                Err(error) => {
                    error!("Couldn't open dmx interface: {}", error);
                    eprintln!("Exiting...");
                    std::process::exit(1);
                },
            };
            if args.options.remember {
                debug!("Setting dmx interface to remember mode");
                dmx.set_async();
            }
            info!("Started!");
            info!("Starting art-net listener...");

            debug!("Creating art-net poll reply packet...");
            let output = match args.options.controller.is_some() {
                true => 0x8A,
                false => 0x80,
            };
            let mut short_name = [0; 18];
            "artnet2opendmx".bytes().enumerate().for_each(|(i, b)| short_name[i] = b);
            let mut long_name = [0; 64];
            match &args.options.name {
                Some(name) if name.as_bytes().len() <= 64 => name.clone(),
                _ => "artnet_to_opendmx_node".into(),
            }.as_bytes().iter().zip(long_name.iter_mut()).for_each(|(a, b)| *b = *a);

            let poll_reply = PollReply {
                address: [0, 0, 0, 0].into(),
                port: 0,
                version: [1, 0],
                port_address: args.universe.to_be_bytes(),
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

            let artnet_output = ArtnetRecieverBuilder::default()
                .socket_address(format!("{}:{}", args.options.controller.clone().unwrap_or("0.0.0.0".into()), args.options.port.unwrap_or(6454)).parse()?)
                .poll_reply(poll_reply)
                .build()?;

            info! ("Started!");
            
            for output in artnet_output {
                if output.port_address == PortAddress::try_from(args.universe).unwrap() {
                    debug!("Received output for universe {}", args.universe);
                    let mut channels = [0; 512];
                    output.to_bytes()?[8..].iter().zip(channels.iter_mut()).for_each(|(a, b)| *b = *a);
                    dmx.set_channels(channels);
                    match dmx.update() {
                        Ok(_) => {},
                        Err(_) => {
                            error!("Couldn't update dmx channels. Interface got disconnected.");
                            debug!("Trying to reconnect...");
                            if let Err(e) = dmx.reopen() {
                                error!("Couldn't reconnect to dmx interface: {}", e);
                            }
                        },
                    }
                    debug!("Updated dmx channels on interface");
                }
            }
            Ok(())
        }   
    }
}