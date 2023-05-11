use std::env;
use artnet_protocol::PortAddress;

pub const HELP_TEXT: &str =
"A simple artnet to opendmx bridge

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
  -c  --controller A specific controller to listen to (localhost is 0.0.0.0) (default: all)
  -p  --port       The port to listen to (default: 6454)
  -n  --name       The name of the node
      --remember   Keep the last dmx values if the art-net connection is lost (default: false)
      --verbose    Print information about the received art-net packets       (default: false)";

///A tool for controlling an open dmx interface via art-net
#[derive(Debug)]
pub enum Command {
    List,
    Help,
    Version,
    Cli(Arguments),
    Gui(Option<Arguments>),
}

impl Command {
    pub fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let args = env::args().collect::<Vec<String>>();
        let mut args = args.into_iter();
        let _ = args.next(); //remove the first argument (the program name)
        let command = match args.next() {
            Some(command) => command,
            None => return Ok(Self::Gui(None)),
        };
        if command.parse::<u16>().is_ok() {
            //Default Command
            if args.len() < 1 {
                return Err("Not enough arguments".into());
            }
            _ = PortAddress::try_from(command.parse::<u16>()?)?;
            let universe = command.parse::<u16>()?;
            let device_name = args.next().unwrap();
            //check for options
            let mut options = Options::default();
            let mut args2 = args.clone();
            _ = args2.next();
            let mut skip = false;
            let mut gui = true;
            for arg in args {
                if skip {
                    skip = false;
                    continue;
                }
                match arg.as_str() {
                    "-p" | "--port" => {
                        if args2.len() < 1 {
                            return Err("Not enough arguments".into());
                        }
                        options.port = Some(args2.next().unwrap().parse::<>()?);
                        skip = true;
                    },
                    "-c" | "--controller" => {
                        if args2.len() < 1 {
                            return Err("Not enough arguments".into());
                        }
                        options.controller = Some(args2.next().unwrap());
                        skip = true;
                    },
                    "-n" | "--name" => {
                        if args2.len() < 1 {
                            return Err("Not enough arguments".into());
                        }
                        options.name = Some(args2.next().unwrap());
                        skip = true;
                    },
                    "--remember" => options.remember = true,
                    "--verbose" => options.verbose = true,
                    "--nogui" => gui = false,
                    _ => {
                        return Err(format!("Unknown option \"{arg}\"").into());
                    }
                }
                args2.next();
            }
            let args = Arguments {
                universe,
                device_name,
                options,
            };
            return Ok(if gui {
                Self::Gui(Some(args))
            } else {
                Self::Cli(args)
            });
        }
        //Other command
        match command.as_str() {
            "list" | "-L" | "-l" | "--list" => Ok(Self::List),
            "help" | "-H" | "-h" | "--help" => Ok(Self::Help),
            "version" | "-V" | "-v" | "--version" => Ok(Self::Version),
            _ => {
                Err(format!("Unknown command \"{command}\"").into())
            }
        }
    }
}

#[derive(Debug)]
pub struct Arguments {
    ///The art-net universe to listen to
    pub universe: u16,
    ///The interface port name
    pub device_name: String,
    
    pub options: Options,
}

#[derive(Debug, Default)]
pub struct Options {
    ///The port to listen to (default: 6454)
    pub port: Option<u16>,
    ///A specific controller to listen to (default: all)
    pub controller: Option<String>,
    ///The name of the node
    pub name: Option<String>,
    ///Keep the last dmx values if the art-net connection is lost (default: false)
    pub remember: bool,
    ///Print information about the received art-net packets (default: false)
    pub verbose: bool,
}