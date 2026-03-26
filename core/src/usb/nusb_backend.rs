use color_eyre::eyre::{Context, ContextCompat, eyre};
use nusb::{
    Interface,
    transfer::{Bulk, In, Out},
};
use std::time::Duration;

use super::{UsbBackend, UsbError};

pub struct NusbBackend {
    _interface: Interface,
    ep_in: nusb::transfer::Endpoint<Bulk, In>,
    ep_out: nusb::transfer::Endpoint<Bulk, Out>,
}

impl NusbBackend {
    pub fn new() -> color_eyre::Result<Self> {
        let device_info = nusb::list_devices()
            .wait()?
            .find(|dev| dev.vendor_id() == 0x57e && dev.product_id() == 0x3000)
            .wrap_err(
                [
                    "Unable to discover Nintendo Switch through USB.",
                    "Ensure the Nintendo Switch is awake and connected via cable to this computer.",
                ]
                .join("\n"),
            )?;

        let device = device_info
            .open()
            .wait()
            .map_err(|e| match (e.kind(), e.os_error()) {
                (nusb::ErrorKind::PermissionDenied, _) | (nusb::ErrorKind::Other, Some(13)) => {
                    eyre!(
                        "Permission denied opening USB connection to Nintendo Switch\nEnsure you have read-write permissions for USB device at bus {} and address {}",
                        device_info.bus_id(),
                        device_info.device_address(),
                    )
                }
                _ => eyre!("Failed to open USB connection to Nintendo Switch: {:?}", e),
            })?;

        let interface = device.claim_interface(0).wait()?;
        let ep_out = interface.endpoint::<Bulk, Out>(0x01)?;
        ep_out.clear_halt().wait()?;
        let ep_in = interface.endpoint::<Bulk, In>(0x81)?;
        ep_in.clear_halt().wait()?;

        Ok(Self {
            _interface: interface,
            ep_in,
            ep_out,
        })
    }
}

impl UsbBackend for NusbBackend {
    fn read_bulk(&mut self, len: usize, timeout: Duration) -> Result<Vec<u8>, UsbError> {
        let buf = nusb::transfer::Buffer::new(len);
        let result = self.ep_in.transfer_blocking(buf, timeout).into_result();

        result.map(|b| b.to_vec()).map_err(|e| match e {
            nusb::transfer::TransferError::Cancelled => UsbError::Cancelled,
            nusb::transfer::TransferError::Disconnected => UsbError::Disconnected,
            nusb::transfer::TransferError::Fault
            | nusb::transfer::TransferError::Stall
            | nusb::transfer::TransferError::InvalidArgument => {
                UsbError::Protocol(format!("{:?}", e))
            }
            nusb::transfer::TransferError::Unknown(i) => {
                UsbError::Other(format!("Unknown error {}", i))
            }
        })
    }

    fn write_bulk(&mut self, data: &[u8], timeout: Duration) -> Result<(), UsbError> {
        let result = self.ep_out.transfer_blocking(data.to_vec(), timeout);

        result.status.map_err(|e| match e {
            nusb::transfer::TransferError::Cancelled => UsbError::Cancelled,
            nusb::transfer::TransferError::Disconnected => UsbError::Disconnected,
            nusb::transfer::TransferError::Fault
            | nusb::transfer::TransferError::Stall
            | nusb::transfer::TransferError::InvalidArgument => {
                UsbError::Protocol(format!("{:?}", e))
            }
            nusb::transfer::TransferError::Unknown(i) => {
                UsbError::Other(format!("Unknown error {}", i))
            }
        })
    }
}
