# ironfoil

A tiny command-line tool to transfer to [Awoo installer](https://github.com/Huntereb/Awoo-Installer).

![demo gif](media/demo.gif)

## Installation

As of now, installation from source is the only options.

You have two alternatives:

1. Install using cargo

   ```sh
   cargo install --git https://github.com/sermuns/ironfoil
   ```

2. Manually clone then install
   ```sh
   git clone https://github.com/sermuns/ironfoil
   cd ironfoil
   cargo install --path cli/
   ```

Both of these methods places `ironfoil` in `~/.cargo/bin/`, which normally should be part of your `$PATH`.

## Usage

```present cargo run -- -h
CLI alternative to NS-USBloader. Transfer data to Awoo Installer.

Usage: ironfoil <GAME_BACKUP_PATH> <COMMAND>

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
