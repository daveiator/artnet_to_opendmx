use serialport::{available_ports, SerialPort};
use open_dmx::DMXSerial;

use std::env;

const HELP_TEXT: &str =
"A tool for controlling an open dmx interface via art-net

Usage: artnet_to_opendmx.exe <UNIVERSE> <DEVICE_NAME> [OPTIONS]
       artnet_to_opendmx.exe <COMMAND>

Commands:
  list    List available devices
  help    Print this message
  version Print version

Arguments:
  <UNIVERSE>     The art-net universe to listen to
  <DEVICE_NAME>  The interface port name

Options:
      --remember  Keep the last dmx values if the art-net connection is lost (default: false)
      --verbose   Print information about the received art-net packets       (default: false)";

///A tool for controlling an open dmx interface via art-net
#[derive(Debug)]
struct Cli {
    command: Command,
}

impl Cli {
    fn parse() -> Self {
        let args = env::args().collect::<Vec<String>>();
        let mut args = args.into_iter();
        let _ = args.next(); //remove the first argument (the program name)
        let command = args.next().expect("Not enough arguments");
        if command.parse::<u16>().is_ok() {
            //Default Command
            if args.len() < 1 {
                eprintln!("Not enough arguments");
                eprintln!("Exiting...");
                std::process::exit(1);
            }
            let universe = command.parse::<u16>().unwrap();
            let device_name = args.next().unwrap();
            //check for options
            let mut options = Options::default();
            for arg in args {
                match arg.as_str() {
                    "--remember" => options.remember = true,
                    "--verbose" => options.verbose = true,
                    _ => {
                        eprintln!("Unknown option \"{}\"", arg);
                        eprintln!("Exiting...");
                        std::process::exit(1);
                    }
                }
            }
            return Self {
                command: Command::Default(Arguments {
                    universe,
                    device_name,
                    options,
                }),
            }
        }
        //Other command
        match command.as_str() {
            "list" | "-L" | "-l" | "--list" => Self {
                command: Command::List,
            },
            "help" | "-H" | "-h" | "--help" => Self {
                command: Command::Help,
            },
            "version" | "-V" | "-v" | "--version" => Self {
                command: Command::Version,
            },
            _ => {
                eprintln!("Unknown command \"{}\"", command);
                eprintln!("Exiting...");
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug)]
enum Command {
    List,
    Help,
    Version,
    Default(Arguments),
}

#[derive(Debug)]
struct Arguments {
    ///The art-net universe to listen to
    universe: u16,
    ///The interface port name
    device_name: String,
    
    options: Options,
}

#[derive(Debug, Default)]
struct Options {
    ///Keep the last dmx values if the art-net connection is lost (default: false)
    remember: bool,
    ///Print information about the received art-net packets (default: false)
    verbose: bool,
}

fn main() {
    let args = Cli::parse();
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
        },
        Command::Help => println!("{}", HELP_TEXT),
        Command::Version => println!("artnet_to_opendmx 0.1.0"),

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
            println!("Starting art-net listener...");
            loop {
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        }   
    }
}


