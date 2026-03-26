use color_eyre::eyre::{Context, ContextCompat};
use log::{debug, info};
use rusb::{Context as RusbContext, DeviceHandle, UsbContext as _};
use std::time::Duration;

use super::{UsbBackend, UsbError};

pub struct RusbBackend {
    handle: DeviceHandle<RusbContext>,
}

impl RusbBackend {
    pub fn new() -> color_eyre::Result<Self> {
        let context = RusbContext::new()?;
        let devices = context.devices()?;

        let device_list: Vec<_> = devices.iter().collect();
        debug!("rusb found {} USB device(s)", device_list.len());

        for (idx, dev) in device_list.iter().enumerate() {
            if let Ok(desc) = dev.device_descriptor() {
                debug!(
                    "  Device {}: vendor_id=0x{:04x}, product_id=0x{:04x}, bus={}, address={}",
                    idx,
                    desc.vendor_id(),
                    desc.product_id(),
                    dev.bus_number(),
                    dev.address()
                );
            }
        }

        let device = device_list
            .iter()
            .find(|dev| {
                let desc = dev.device_descriptor().ok();
                desc.is_some_and(|d| d.vendor_id() == 0x57e && d.product_id() == 0x3000)
            })
            .wrap_err(
                [
                    "Unable to discover Nintendo Switch through USB (rusb).",
                    "Ensure the Nintendo Switch is awake and connected via cable to this computer.",
                ]
                .join("\n"),
            )?;

        info!(
            "Found Nintendo Switch at bus {} address {}",
            device.bus_number(),
            device.address()
        );

        let handle = device.open().map_err(|e| {
            let base_msg = format!(
                "Failed to open USB connection to Nintendo Switch (rusb) at bus {} and address {}",
                device.bus_number(),
                device.address(),
            );

            match e {
                rusb::Error::Access => {
                    color_eyre::eyre::eyre!(
                        [
                            &base_msg,
                            "",
                            "The Nintendo Switch device exists but cannot be accessed.",
                            "You need to install a USB driver (WinUSB or libusb-k) for the device.",
                            "",
                            "Steps to fix:",
                            "1. Download Zadig: https://zadig.akeo.ie/",
                            "2. Open Zadig and select your Nintendo Switch device",
                            "3. Select 'WinUSB' or 'libusb-k' as the driver",
                            "4. Click 'Install Driver'",
                            "5. Retry this application",
                            "",
                            "Once the driver is installed, no admin privileges are needed.",
                        ]
                        .join("\n")
                    )
                }
                _ => color_eyre::eyre::eyre!("{}: {:?}", base_msg, e),
            }
        })?;

        // In rusb, we usually need to check if a kernel driver is active and detach it on Unix,
        // but on Windows (where this backend is primarily intended), this is handled by the driver.
        // We claim interface 0.
        handle
            .claim_interface(0)
            .wrap_err("Failed to claim USB interface 0")?;

        // Clear halt on endpoints
        let _ = handle.clear_halt(0x01);
        let _ = handle.clear_halt(0x81);

        Ok(Self { handle })
    }
}

impl UsbBackend for RusbBackend {
    fn read_bulk(&mut self, len: usize, timeout: Duration) -> Result<Vec<u8>, UsbError> {
        let mut buf = vec![0u8; len];
        match self.handle.read_bulk(0x81, &mut buf, timeout) {
            Ok(n) => {
                buf.truncate(n);
                Ok(buf)
            }
            Err(rusb::Error::Timeout) => {
                // Return empty or error? The original nusb logic handles timeout in the transfer result.
                // We'll treat it as a protocol/other error if it wasn't expected.
                Err(UsbError::Other("USB read timeout".to_string()))
            }
            Err(rusb::Error::NoDevice) => Err(UsbError::Disconnected),
            Err(rusb::Error::Interrupted) => Err(UsbError::Cancelled),
            Err(e) => Err(UsbError::Protocol(format!("rusb read error: {:?}", e))),
        }
    }

    fn write_bulk(&mut self, data: &[u8], timeout: Duration) -> Result<(), UsbError> {
        match self.handle.write_bulk(0x01, data, timeout) {
            Ok(_) => Ok(()),
            Err(rusb::Error::Timeout) => Err(UsbError::Other("USB write timeout".to_string())),
            Err(rusb::Error::NoDevice) => Err(UsbError::Disconnected),
            Err(rusb::Error::Interrupted) => Err(UsbError::Cancelled),
            Err(e) => Err(UsbError::Protocol(format!("rusb write error: {:?}", e))),
        }
    }
}
