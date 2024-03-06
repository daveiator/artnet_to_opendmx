# artnet_to_opendmx &emsp; [![Latest Release][crates-io-badge]][crates-io-url] [![Build][build-badge]]() [![License][license-badge]]()

[crates-io-badge]: https://img.shields.io/crates/v/artnet_to_opendmx.svg?style=for-the-badge
[crates-io-url]: https://crates.io/crates/artnet_to_opendmx
[build-badge]: https://img.shields.io/github/actions/workflow/status/daveiator/artnet_to_opendmx/build.yml?style=for-the-badge
[license-badge]: https://img.shields.io/crates/l/artnet_to_opendmx.svg?style=for-the-badge

 ### A simple artnet to opendmx bridge

<img src="assets/logo.svg" width="200" height="200" />

<br>

**Works with both COM- and /dev/tty-Ports.**

## Usage:
```bash
artnet_to_opendmx.exe <UNIVERSE> <DEVICE_NAME> [OPTIONS]
artnet_to_opendmx.exe <COMMAND>
```

| __Commands__ | |
| - | - |
| **list** | List available devices |
| **help** | Print a message |
| **version** | Print version |


| __Arguments__ | |
| - | - |
| < UNIVERSE > | The art-net universe to listen to |
| < DEVICE_NAME > | The interface port name |

| __Options__ | | |
| - | - | - |
| -c | --controller | A specific controller to listen to (localhost is 0.0.0.0) (default: all) |
| -p | --port | The port to listen to (default: 6454) |
| -n | --name | The name of the node |
| -b | --break | The minimum time in milliseconds between two dmx packets (default: 25) |
| -r | --remember | Keep the last dmx values if the art-net connection is lost (default: false) |
| | --verbose | Print information about the received art-net packets       (default: false) |
| | --nogui | Disable the GUI (default: false) |

## Example:
#### Opens a bridge named "Interface1" on universe 0 and the device COM4
```bash
artnet_to_opendmx.exe 0 COM4 --name "Interface1" --remember --verbose
```

#### List all available devices
```bash
artnet_to_opendmx.exe list
```

## Troubleshooting
* **Settings-Window has scaling issues**
    
    Check if the application has the permission to scale the window. This should only be a problem on linux.

* **Flickering DMX output**
    
    If the DMX output flickers, try to increase the break time. This can happen if the DMX-Interface is not able to handle the data rate.

    If multiple senders are sending data to the same universe, the data might interfere. Try to set the controller option to a specific sender.

* **Anything else?**

    Please open an issue if you encounter any other problems.

## Contributions
Contributions are welcome! If you have something that could improve the program, please open an issue or a pull request.
