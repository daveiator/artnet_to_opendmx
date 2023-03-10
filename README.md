# artnet_to_opendmx &emsp; [![Latest Release][crates-io-badge]][crates-io-url] [![Documentation][docs-rs-img]][docs-rs-url] [![License][license-badge]]()

[crates-io-badge]: https://img.shields.io/crates/v/artnet_to_opendmx.svg?style=for-the-badge
[crates-io-url]: https://crates.io/crates/artnet_to_opendmx
[docs-rs-img]: https://img.shields.io/docsrs/artnet_to_opendmx?style=for-the-badge
[docs-rs-url]: https://docs.rs/artnet_to_opendmx
[license-badge]: https://img.shields.io/crates/l/artnet_to_opendmx.svg?style=for-the-badge

 ### A simple artnet to opendmx bridge


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
| <UNIVERSE> | The art-net universe to listen to |
| <DEVICE_NAME> | The interface port name |

| __Options__ | | |
| - | - | - |
| -c | --controller | A specific controller to listen to (localhost is 0.0.0.0) (default: all) |
| -p | --port | The port to listen to (default: 6454) |
| -n | --name | The name of the node |
| | --remember | Keep the last dmx values if the art-net connection is lost (default: false) |
| | --verbose | Print information about the received art-net packets       (default: false) |

_Should work cross-platform, but only tested on Windows._

## Example:
#### Opens a bridge named "Interface1" on universe 0 and the device COM4
```bash
artnet_to_opendmx.exe 0 COM4 --name "Interface1" --remember --verbose
```

#### List all available devices
```bash
artnet_to_opendmx.exe list
```

## Contributions
Contributions are welcome! If you have something that could improve the program, please open an issue or a pull request.