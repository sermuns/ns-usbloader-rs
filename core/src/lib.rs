mod network;
mod paths;
mod rcm;
mod usb;

#[derive(Debug)]
pub enum InstallProgressEvent {
    /// Request to show status message to user.
    CurrentFileName(String),
    /// Installation has ended, either successfully or by error. Check thread return value!
    Ended,
    /// Total size of all requested files. Should only be sent once on install start.
    AllFilesLengthBytes(u64),
    /// How far we've gotten through all files, totally.
    AllFilesOffsetBytes(u64),
    /// The size of current file being installed. Should only be sent on start of this file.
    CurrentFileLengthBytes(u64),
    /// How far we've gotten through current file.
    CurrentFileOffsetBytes(u64),
}
pub type InstallProgressSender = std::sync::mpsc::Sender<InstallProgressEvent>;
pub type InstallProgressReceiver = std::sync::mpsc::Receiver<InstallProgressEvent>;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum UsbProtocol {
    #[default]
    TinFoil,
    Sphaira,
}

pub use network::perform_tinfoil_network_install;
pub use paths::{GAME_BACKUP_EXTENSIONS, RCM_PAYLOAD_EXTENSIONS, read_game_paths};
pub use rcm::send_rcm_payload;
pub use usb::perform_usb_install;
