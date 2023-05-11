mod cli;
use cli::{Command, HELP_TEXT};

mod runner;

use serialport::{available_ports};

use simple_logger::SimpleLogger;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = match Command::parse() {
        Ok(command) => command,
        Err(error) => {
            eprintln!("Couldn't parse arguments:\n{error}");
            eprintln!("Exiting...");
            std::process::exit(1);
        },
    };
    match command {
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

        Command::Cli(args) => {
            log_panics::init();
            SimpleLogger::new()
                .with_level(match args.options.verbose {
                    true => log::LevelFilter::Debug,
                    false => log::LevelFilter::Info,
                })
                .without_timestamps()
                .with_colors(true)
                .init()?;

            let runner_update_reciever = match runner::create_runner(args) {
                Ok(runner_update_reciever) => runner_update_reciever,
                Err(error) => {
                    eprintln!("Couldn't create runner: {}", error);
                    eprintln!("Exiting...");
                    std::process::exit(1);
                },
            };

            for update in runner_update_reciever {
                println!("Recieved update: {:?}", update);
                std::thread::sleep_ms(1000);
            }
            Ok(())
        }
        Command::Gui(argument_option) => {
            unimplemented!("GUI not implemented yet");
            Ok(())
        }
        _ => {
            eprintln!("Invalid command");
            eprintln!("Exiting...");
            std::process::exit(1);
        },
    }
}