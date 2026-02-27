use color_eyre::eyre::ContextCompat;
use log::info;
use nusb::transfer::{Bulk, ControlOut, ControlType, In, Out, Recipient, TransferError};
use nusb::{MaybeFuture, list_devices};
use std::io::{Error, ErrorKind, Read, Write};
use std::time::Duration;

const USB_TIMEOUT: Duration = Duration::from_millis(5050);

fn write_usb(message: &[u8], interface: &mut nusb::Interface) -> std::io::Result<()> {
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    env_logger::init();
    color_eyre::install()?;

    let device_info = list_devices()
        .wait()?
        .find(|dev| dev.vendor_id() == 0x57e && dev.product_id() == 0x3000)
        .wrap_err("unable to discover NS through USB")?;

    info!(
        "NS discovered at bus {} and address {}",
        device_info.bus_id(),
        device_info.device_address()
    );

    const HOMEBREW_CONFIGURATION: u8 = 1;
    let device = device_info.open().wait()?;
    device.set_configuration(HOMEBREW_CONFIGURATION).wait()?; // TODO: check if needed?

    let interface = device.claim_interface(0).wait()?;

    const PADDING: &[u8] = &[0u8; 8];
    let message = [
        "TUL0".as_bytes(),
        &1u32.to_le_bytes(),
        PADDING,
        "test\n".as_bytes(),
    ]
    .concat();
    // println!("message: {:?}", &message);

    const OUT_ENDPOINT_ADDRESS: u8 = 0x01;
    let mut writer = interface
        .endpoint::<Bulk, Out>(OUT_ENDPOINT_ADDRESS)?
        .writer(128); // NOTE: how to choose buffer size?

    writer.write_all(&message)?;
    writer.with_write_timeout(USB_TIMEOUT).flush()?;

    const IN_ENDPOINT_ADDRESS: u8 = 0x81;
    let mut reader = interface
        .endpoint::<Bulk, In>(IN_ENDPOINT_ADDRESS)?
        .reader(128); // NOTE: how to choose buffer size?
    const MAGIC: &[u8] = "TUC0".as_bytes();
    let mut buf = [0; MAGIC.len()];
    println!("start reading");
    reader.read_exact(&mut buf)?;
    println!("buf: {:?}", buf);

    Ok(())
}
