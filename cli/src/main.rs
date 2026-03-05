use clap::{Args, Parser, Subcommand};
use ironfoil_core::{
    perform_tinfoil_network_install, perform_tinfoil_usb_install, send_rcm_payload,
};
use std::{net::Ipv4Addr, path::PathBuf};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    transfer_type: TransferType,
}

#[derive(Debug, Args)]
struct TransferArgs {
    /// Path to a game backup file or directory containing game backup files
    game_backup_path: PathBuf,

    /// Whether to recursively look for files (only for directories)
    #[arg(short, long)]
    recurse: bool,
}

#[derive(Debug, Subcommand)]
enum TransferType {
    /// Transfer over USB
    #[command(arg_required_else_help = true)]
    Usb {
        #[command(flatten)]
        transfer_args: TransferArgs,
    },

    /// Transfer over network
    #[command(arg_required_else_help = true)]
    Network {
        #[command(flatten)]
        transfer_args: TransferArgs,

        /// The IP address of the Nintendo Switch
        target_ip: Ipv4Addr,
    },

    /// Inject RCM payload
    #[command(arg_required_else_help = true)]
    Rcm {
        /// Path to the RCM payload file
        payload_path: PathBuf,
    },
}

fn main() -> color_eyre::Result<()> {
    env_logger::builder().format_source_path(true).init();
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .display_location_section(cfg!(debug_assertions))
        .install()?;

    let args = Cli::parse();

    match args.transfer_type {
        TransferType::Usb { transfer_args } => {
            perform_tinfoil_usb_install(&transfer_args.game_backup_path, transfer_args.recurse)
        }
        TransferType::Network {
            transfer_args,
            target_ip,
        } => perform_tinfoil_network_install(
            &transfer_args.game_backup_path,
            transfer_args.recurse,
            target_ip,
        ),
        TransferType::Rcm { payload_path } => send_rcm_payload(&payload_path),
    }
}
