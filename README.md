# ns-usbloader-rs

A proof-of-concept command line re-implementation of [NS-USBloader](https://github.com/developersu/ns-usbloader). Only NSPs are supported for now.

## Installation

When I can think of a better name than `ns-usbloader-rs` and polished the code a bit more, I will probably publish this as a crate and publish binaries.

As of now, only installation from source is available. You have two alternatives:

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

1. On the Nintendo Switch, open Awoo Installer and select 'Install over USB'.

2. On your PC, in a shell, run `ns-usbloader-rs <GAME_BACKUP_DIR>` where `<GAME_BACKUP_DIR>` is the path to a directory containing the game backups.

   > For example
   >
   > ```sh
   > ns-usbloader-rs ~/Documents/Backups
   > ```

3. On the Nintendo Switch, select which game(s) you want, then start installation.

4. .. profit?

Here's the output of `ns-usbloader-rs --help`:

```present cargo run -- -h
Usage: ns-usbloader-rs <GAME_BACKUP_DIR>

Arguments:
  <GAME_BACKUP_DIR>

Options:
  -h, --help  Print help
```
