use color_eyre::eyre::{ContextCompat, bail, eyre};
use log::info;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    time::Duration,
};
use thiserror::Error;

#[cfg(not(windows))]
mod nusb_backend;
#[cfg(windows)]
mod rusb_backend;

const USB_TIMEOUT: Duration = Duration::from_millis(500);

/// Abstract representation of USB transfer errors to unify backends
#[derive(Debug, Error)]
pub enum UsbError {
    #[error("Transfer cancelled")]
    Cancelled,
    #[error("Device disconnected")]
    Disconnected,
    #[error("Malformed data or protocol error: {0:?}")]
    Protocol(String),
    #[error("Unknown error: {0}")]
    Other(String),
}

/// Minimal trait to abstract over nusb and rusb blocking transfers
pub trait UsbBackend {
    fn read_bulk(&mut self, len: usize, timeout: Duration) -> Result<Vec<u8>, UsbError>;
    fn write_bulk(&mut self, data: &[u8], timeout: Duration) -> Result<(), UsbError>;
}

mod tinfoil {
    use super::*;
    use std::path::PathBuf;

    pub mod command_direction {
        pub const RESPONSE: [u8; 4] = 0u32.to_le_bytes();
    }
    pub mod command {
        pub const EXIT: [u8; 4] = 0u32.to_le_bytes();
        pub const FILE_RANGE: [u8; 4] = 1u32.to_le_bytes();
    }

    pub fn send_response_header(
        backend: &mut dyn UsbBackend,
        range_size: usize,
    ) -> color_eyre::Result<()> {
        write_usb(backend, b"TUC0")?;
        write_usb(backend, tinfoil::command_direction::RESPONSE)?;
        write_usb(backend, tinfoil::command::FILE_RANGE)?;
        write_usb(backend, range_size.to_le_bytes())?;
        write_usb(backend, [0u8; 0xC])?;
        Ok(())
    }

    pub fn file_range_command(
        backend: &mut dyn UsbBackend,
        game_paths: &[PathBuf],
        progress_len_tx: &mpsc::Sender<u64>,
        progress_tx: &mpsc::Sender<u64>,
    ) -> color_eyre::Result<()> {
        let file_range_header = read_usb(backend)?;

        let range_size = usize::from_le_bytes(file_range_header[..8].try_into().unwrap());
        let range_offset = u64::from_le_bytes(file_range_header[8..16].try_into().unwrap());
        let game_path_len = usize::from_le_bytes(file_range_header[16..24].try_into().unwrap());

        let game_name_buf = read_usb(backend)?;
        let game_path = std::str::from_utf8(&game_name_buf)?;

        if !game_paths
            .iter()
            .any(|path| game_path == path.to_str().unwrap_or(""))
        {
            bail!(
                "Nintendo Switch tried to request game backup ({}) not present on host",
                game_path
            );
        };

        info!("sending {}", &game_path);
        info!(
            "Range size: {}, Range offset: {}, Name len: {}, Name: {}",
            range_size, range_offset, game_path_len, game_path,
        );

        send_response_header(backend, range_size)?;

        let file = File::open(game_path)?;
        if let Ok(metadata) = file.metadata() {
            progress_len_tx.send(metadata.len())?;
        }

        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::Start(range_offset))?;

        let mut current_offset = 0;
        let end_offset = range_size;
        let mut read_size = 0x100000;
        let mut buf = vec![0u8; read_size];

        while current_offset < end_offset {
            if current_offset + read_size >= end_offset {
                read_size = end_offset - current_offset;
                buf.resize(read_size, 0u8);
            }
            reader.read_exact(&mut buf)?;
            backend
                .write_bulk(&buf, Duration::MAX)
                .map_err(|e| eyre!(e))?;

            current_offset += read_size;
            progress_tx.send(current_offset as u64)?;
        }

        Ok(())
    }

    pub fn do_workloop(
        backend: &mut dyn UsbBackend,
        cancel: Option<Arc<AtomicBool>>,
        game_paths: &[PathBuf],
        progress_len_tx: mpsc::Sender<u64>,
        progress_tx: mpsc::Sender<u64>,
    ) -> color_eyre::Result<()> {
        loop {
            if cancel.as_ref().is_some_and(|c| c.load(Ordering::Relaxed)) {
                return Ok(());
            }

            let command_header = backend
                .read_bulk(512, Duration::MAX)
                .map_err(|e| eyre!(e))?;
            if &command_header[..4] != b"TUC0" {
                continue;
            }

            let command_id: [u8; 4] = command_header[8..12].try_into().unwrap();
            match command_id {
                tinfoil::command::EXIT => break,
                tinfoil::command::FILE_RANGE => tinfoil::file_range_command(
                    backend,
                    game_paths,
                    &progress_len_tx,
                    &progress_tx,
                )?,
                _ => bail!("invalid tinfoil command encountered!"),
            }
        }
        Ok(())
    }
}

mod sphaira {
    use super::*;

    const EXPECTED_MAGIC: u32 = u32::from_be_bytes(*b"SPH0");
    const PACKET_SIZE: usize = 24;
    pub const RESULT_OK: u32 = 0;
    pub const RESULT_ERROR: u32 = 1;

    mod command {
        pub const QUIT: u32 = 0;
        pub const OPEN: u32 = 1;
    }

    struct SendHeader {
        arg2: u32,
        arg3: u32,
        arg4: u32,
    }

    impl SendHeader {
        fn get_offset(&self) -> u64 {
            ((self.arg2 as u64) << 32) | (self.arg3 as u64)
        }
        fn get_size(&self) -> usize {
            self.arg4 as usize
        }
    }

    pub fn validate_send_header(backend: &mut dyn UsbBackend) -> color_eyre::Result<()> {
        let _ = get_send_header(backend)?;
        Ok(())
    }

    fn get_send_header(backend: &mut dyn UsbBackend) -> color_eyre::Result<SendHeader> {
        let response = read_usb(backend)?;
        let mut args = response.chunks(4);
        let [magic, arg2, arg3, arg4, _arg5, crc32c] =
            std::array::from_fn(|_| u32::from_le_bytes(args.next().unwrap().try_into().unwrap()));

        if magic != EXPECTED_MAGIC {
            bail!("Invalid magic in Sphaira header")
        }
        let computed_crc32c = crc32c::crc32c(&response[0..20]);
        if computed_crc32c != crc32c {
            bail!("Invalid CRC32C in Sphaira header")
        }

        Ok(SendHeader { arg2, arg3, arg4 })
    }

    pub fn send_result(
        backend: &mut dyn UsbBackend,
        result: u32,
        arg3: Option<u32>,
        arg4: Option<u32>,
    ) -> color_eyre::Result<()> {
        let mut packet = [0u8; PACKET_SIZE];
        packet[..4].copy_from_slice(&EXPECTED_MAGIC.to_le_bytes());
        packet[4..8].copy_from_slice(&result.to_le_bytes());
        packet[8..12].copy_from_slice(&arg3.unwrap_or(0).to_le_bytes());
        packet[12..16].copy_from_slice(&arg4.unwrap_or(0).to_le_bytes());
        let crc32c = crc32c::crc32c(&packet[0..20]);
        packet[20..24].copy_from_slice(&crc32c.to_le_bytes());
        write_usb(backend, packet)
    }

    fn do_file_transfer_loop(
        backend: &mut dyn UsbBackend,
        cancel: Option<&AtomicBool>,
        mut file_reader: impl Read + Seek,
        progress_tx: &mpsc::Sender<u64>,
    ) -> color_eyre::Result<()> {
        loop {
            if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                send_result(backend, RESULT_ERROR, None, None)?;
                return Ok(());
            }

            let send_header = match get_send_header(backend) {
                Ok(header) => header,
                Err(e) => {
                    send_result(backend, RESULT_ERROR, None, None)?;
                    return Err(e);
                }
            };

            let offset = send_header.get_offset();
            let size = send_header.get_size();
            let _ = progress_tx.send(offset);

            if offset == 0 && size == 0 {
                send_result(backend, RESULT_OK, None, None)?;
                break;
            }

            file_reader.seek(SeekFrom::Start(offset))?;
            let mut buf = vec![0u8; size];
            let bytes_read = file_reader.read(&mut buf)?;
            send_result(
                backend,
                RESULT_OK,
                Some(bytes_read as u32),
                Some(crc32c::crc32c(&buf)),
            )?;
            write_usb(backend, buf)?;
        }
        Ok(())
    }

    pub fn do_workloop(
        backend: &mut dyn UsbBackend,
        cancel: Option<Arc<AtomicBool>>,
        game_paths: &[PathBuf],
        progress_len_tx: mpsc::Sender<u64>,
        progress_tx: mpsc::Sender<u64>,
    ) -> color_eyre::Result<()> {
        loop {
            let header = match get_send_header(backend) {
                Ok(h) => h,
                Err(e) => return Err(e),
            };

            match header.arg2 {
                command::QUIT => {
                    send_result(backend, RESULT_OK, None, None)?;
                    break;
                }
                command::OPEN => {
                    let game_path = game_paths
                        .get(header.arg3 as usize)
                        .wrap_err("Invalid game index")?;
                    let file_size = game_path.metadata()?.len();
                    let _ = progress_len_tx.send(file_size);

                    send_result(
                        backend,
                        RESULT_OK,
                        Some(((file_size >> 32) as u16) as u32),
                        Some(file_size as u32),
                    )?;
                    let file = File::open(game_path)?;
                    do_file_transfer_loop(
                        backend,
                        cancel.as_deref(),
                        BufReader::new(file),
                        &progress_tx,
                    )?;
                }
                _ => bail!("Invalid Sphaira command: {}", header.arg2),
            }
        }
        Ok(())
    }
}

pub fn perform_usb_install(
    game_paths: &[PathBuf],
    progress_len_tx: mpsc::Sender<u64>,
    progress_tx: mpsc::Sender<u64>,
    for_sphaira: bool,
    cancel: impl Into<Option<Arc<AtomicBool>>>,
) -> color_eyre::Result<()> {
    let cancel = cancel.into();
    let paths_string_length: u32 = game_paths
        .iter()
        .map(|p| p.to_str().unwrap().len() as u32 + 1)
        .sum();

    #[cfg(not(windows))]
    let mut backend = nusb_backend::NusbBackend::new()?;
    #[cfg(windows)]
    let mut backend = rusb_backend::RusbBackend::new()?;

    if for_sphaira {
        sphaira::validate_send_header(&mut backend)?;
        sphaira::send_result(
            &mut backend,
            sphaira::RESULT_OK,
            Some(paths_string_length),
            None,
        )?;
        write_usb(
            &mut backend,
            game_paths
                .iter()
                .fold(String::new(), |acc, p| acc + p.to_str().unwrap() + "\n"),
        )?;
    } else {
        write_usb(&mut backend, "TUL0")?;
        write_usb(&mut backend, paths_string_length.to_le_bytes())?;
        write_usb(&mut backend, [0u8; 8])?;
        for path in game_paths {
            write_usb(&mut backend, [path.to_str().unwrap(), "\n"].concat())?;
        }
    }

    if for_sphaira {
        sphaira::do_workloop(
            &mut backend,
            cancel,
            game_paths,
            progress_len_tx,
            progress_tx,
        )?;
    } else {
        tinfoil::do_workloop(
            &mut backend,
            cancel,
            game_paths,
            progress_len_tx,
            progress_tx,
        )?;
    }

    Ok(())
}

fn read_usb(backend: &mut dyn UsbBackend) -> color_eyre::Result<Vec<u8>> {
    backend.read_bulk(512, USB_TIMEOUT).map_err(|e| eyre!(e))
}

fn write_usb(backend: &mut dyn UsbBackend, message: impl Into<Vec<u8>>) -> color_eyre::Result<()> {
    backend
        .write_bulk(&message.into(), USB_TIMEOUT)
        .map_err(|e| match e {
            UsbError::Cancelled => eyre!("Transfer cancelled"),
            UsbError::Disconnected => eyre!("USB has disconnected"),
            _ => eyre!(e),
        })
}
