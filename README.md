# ns-usbloader-rs

A tiny command-line re-implementation of [NS-USBloader](https://github.com/developersu/ns-usbloader).

![demo gif](media/demo.gif)

## Installation

When I can think of a better name than `ns-usbloader-rs` ([help me!](https://github.com/sermuns/ns-usbloader-rs/issues/1)) and have polished the code a bit more, I will publish this as a crate and start publishing binaries.

As of now, installation from source is the only options.

You have two alternatives:

1. Install using cargo

   ```sh
   cargo install --git https://github.com/sermuns/ns-usbloader-rs
   ```

2. Manually clone then install
   ```sh
   git clone https://github.com/sermuns/ns-usbloader-rs
   cd ns-usbloader-rs
   cargo install --path .
   ```

Both of these methods places `ns-usbloader-rs` in `~/.cargo/bin/`, which normally should be part of your `$PATH`.

## Usage

```present cargo run -- -h
CLI alternative to NS-USBloader. Transfer data to Awoo Installer.

Usage: ns-usbloader-rs <GAME_BACKUP_PATH> <COMMAND>

Commands:
  usb      Transfer over USB
  network  Transfer over network
  help     Print this message or the help of the given subcommand(s)

Arguments:
  <GAME_BACKUP_PATH>  Path to a game backup file or directory containing game backup files

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## License

Dual-licensed under [Apache 2.0](./LICENSE-APACHE) or [MIT](./LICENSE-MIT).
