mod cli;
use cli::*;


use serialport::{available_ports};
use open_dmx::DMXSerial;
use artnet_protocol::{ArtCommand, PortAddress};

use std::net::{UdpSocket, SocketAddr};

use socket2::{Domain, Socket, Type, Protocol};


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = match Cli::parse() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("Couldn't parse arguments:\n{}", error);
            eprintln!("Exiting...");
            std::process::exit(1);
        },
    };
    match args.command {
        Command::List => {
            println!("Available ports:");
            let ports = match available_ports() {
                Err(error) => {
                    eprintln!("Coulnd't get available ports list: {}", error);
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
            println!("{}", HELP_TEXT);
            Ok(())
        },
        Command::Version => {
            println!("artnet_to_opendmx 0.1.0");
            Ok(())
        },

        Command::Default(args) => {
            println!("Checking for device named \"{}\"...", args.device_name);
            let ports= match available_ports() {
                Err(error) => {
                    eprintln!("Coulnd't get available ports list: {}", error);
                    eprintln!("Exiting...");
                    std::process::exit(1);
                },
                Ok(ports) => ports,
            };
            let mut device = "";
            for port in ports {
                if port.port_name == args.device_name {
                    println!("Found device \"{}\"", args.device_name);
                    device = &args.device_name;
                    break;
                }
            }
            if device == "" {
                eprintln!("Couldn't find device named \"{}\"", args.device_name);
                eprintln!("Exiting...");
                std::process::exit(1);
            }
            println!("Starting dmx interface...");
            let mut dmx = match DMXSerial::open(device) {
                Ok(dmx) => dmx,
                Err(error) => {
                    eprintln!("Couldn't open dmx interface: {}", error);
                    eprintln!("Exiting...");
                    std::process::exit(1);
                },
            };
            println!("Started!");
            println!("Starting art-net listener...");
            let address: SocketAddr = format!("{}:{}", args.options.controller.unwrap_or("0.0.0.0".into()), args.options.port.unwrap_or(6454)).parse()?;
            let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
            socket.set_reuse_address(true)?;
            // socket.set_nonblocking(true)?;
            socket.bind(&address.into())?;
            let socket: UdpSocket = socket.into();

            println!("Started!");
            let mut buffer = [0; 1024];
            loop {
                let (length, controller) = match socket.recv_from(&mut buffer) {
                    Ok(x) => x,
                    Err(error) => {
                        if args.options.verbose {
                            eprintln!("Couldn't receive art-net packet: {}", error);
                        }
                        continue;
                    },
                };
                let command = match ArtCommand::from_buffer(&buffer[..length]) {
                    Ok(command) => command,
                    Err(error) => {
                        if args.options.verbose {
                            eprintln!("Couldn't parse art-net packet: {}", error);
                        }
                        continue;
                    },
                };
                if args.options.verbose {
                    // println!("Received art-net packet: {:?}", command);
                }
                match command {
                    ArtCommand::Poll(_) => {
                        // println!("Received Poll");
                        //TODO Respond to poll
                    },
                    ArtCommand::Output(output) => {
                        if output.port_address == PortAddress::try_from(args.universe).unwrap() {
                            println!("Received output for universe {} from controller {}", args.universe, controller);
                        }
                    }
                    _ => {
                    }
                }
                // std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        }   
    }
}


