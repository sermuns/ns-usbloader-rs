<picture>
  <source media="(prefers-color-scheme: dark)" srcset="media/banner-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="media/banner-light.svg">
  <img alt="ironfoil logo" src="media/banner-light.svg">
</picture>

A tool to transfer to [many title installers](#which-title-installers-are-supported).

![demo gif](media/demo.gif)

## Which title installers are supported?

These, based on the TinFoil USB protocol:

- [Awoo Installer](https://github.com/Huntereb/Awoo-Installer)
- [CyberFoil](https://github.com/luketanti/CyberFoil)

and also:

- [Sphaira](https://github.com/ITotalJustice/sphaira), with its own protocol (make sure you enable Sphaira support when transferring!)

## Installation

### Binary releases

Alternatives:

1. Manually download [latest release](https://github.com/sermuns/ironfoil/releases/latest) and extract the tool to a location that is in your `$PATH`.

2. Use [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall):
   ```sh
   cargo binstall ironfoil
   ```

### From source

Alternatives:

1. Install latest release using cargo

   ```sh
   cargo install ironfoil
   ```

2. Install latest git version using cargo:

   ```sh
   cargo install --git https://github.com/sermuns/ironfoil
   ```

3. Manually clone then install
   ```sh
   git clone https://github.com/sermuns/ironfoil
   cd ironfoil
   cargo install --path cli/
   ```

Both of these methods places `ironfoil` in `~/.cargo/bin/`, which normally should be part of your `$PATH`.

## Usage

```present cargo run --bin ironfoil -- -h
Transfer to NS title installers from the command-line

Usage: ironfoil <COMMAND>

Commands:
  usb      Transfer over USB
  network  Transfer over network
  rcm      Inject RCM payload
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## License

Dual-licensed under [Apache 2.0](./LICENSE-APACHE) or [MIT](./LICENSE-MIT).
