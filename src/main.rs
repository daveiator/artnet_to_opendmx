mod cli;
use cli::{Command, HELP_TEXT};

mod gui;

use gui::run_app;

mod runner;

use log::SetLoggerError;
use serialport::available_ports;

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
            initialize_logger(args.options.verbose)?;
            let runner_update_reciever = match runner::create_runner(args) {
                Ok(runner_update_reciever) => runner_update_reciever,
                Err(error) => {
                    eprintln!("Couldn't create runner: {}", error);
                    eprintln!("Exiting...");
                    std::process::exit(1);
                },
            };

            for _ in runner_update_reciever {
            }
            Ok(())
        }
        Command::Gui(argument_option) => {
            initialize_logger(match &argument_option {
                Some(args) => args.options.verbose,
                None => false,
            })?;
            
            run_app(argument_option)?;

            Ok(())
        }
        // TODO: Fix parsing so this is needed again
        // _ => {
        //     eprintln!("Invalid command");
        //     eprintln!("Exiting...");
        //     std::process::exit(1);
        // },
    }
}

fn initialize_logger(verbose: bool) -> Result<(), SetLoggerError> {
    log_panics::init();
    SimpleLogger::new()
        .with_level(match verbose {
            true => log::LevelFilter::Debug,
            false => log::LevelFilter::Info,
        })
        .without_timestamps()
        .with_colors(true)
    .init()?;
    Ok(())
}