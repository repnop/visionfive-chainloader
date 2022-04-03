use std::io::Write;

use bytemuck::Zeroable;

pub const START_OF_HEADER: u8 = 0x01;
pub const END_OF_TRANSMISSION: u8 = 0x04;
pub const ACK: u8 = 0x06;
pub const NAK: u8 = 0x15;
pub const END_OF_TRANSMISSION_BLOCK: u8 = 0x17;
pub const CANCEL: u8 = 0x18;
pub const CHECKSUM_REQUEST: u8 = b'C';

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Packet {
    r#type: u8,
    id: u8,
    id_inverted: u8,
    data: [u8; 128],
    crc: [u8; 2],
}

pub trait SerialDevice {
    type Error;
    fn read(&mut self) -> Result<u8, Self::Error>;
    fn write(&mut self, c: u8) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub enum Error<S> {
    BadPacketId,
    BadPacketType(u8),
    Canceled,
    Serial(S),
}

impl<S> From<S> for Error<S> {
    fn from(s: S) -> Self {
        Self::Serial(s)
    }
}

impl<S: core::fmt::Display + core::fmt::Debug> std::error::Error for Error<S> {}
impl<S: core::fmt::Display + core::fmt::Debug> core::fmt::Display for Error<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadPacketId => write!(f, "bad XMODEM packet ID"),
            Self::BadPacketType(n) => write!(f, "bad XMODEM packet type {:#x}", n),
            Self::Canceled => write!(f, "XMODEM transfer canceled"),
            Self::Serial(s) => write!(f, "serial error: {}", s),
        }
    }
}

pub struct Sender<S: SerialDevice> {
    buffer: Packet,
    device: S,
}

impl<S: SerialDevice> Sender<S> {
    pub fn new(device: S) -> Self {
        Self {
            buffer: Packet::zeroed(),
            device,
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), Error<S::Error>> {
        let mut id = 1;
        match self.device.read()? {
            CHECKSUM_REQUEST => {}
            ty => return Err(Error::BadPacketType(ty)),
        }

        let chunk_count = data.len() / 128 + 1;
        for (i, chunk) in data.chunks(128).enumerate() {
            self.buffer.r#type = START_OF_HEADER;
            self.buffer.id = id;
            self.buffer.id_inverted = !id;
            self.buffer.data[..chunk.len()].copy_from_slice(chunk);
            self.buffer.crc = u16::to_be_bytes(checksum(&self.buffer.data));

            loop {
                for &byte in bytemuck::bytes_of(&self.buffer) {
                    self.device.write(byte)?;
                }

                match self.device.read()? {
                    ACK => break,
                    NAK => {
                        println!("NAK'd, retrying...");
                        continue;
                    }
                    CANCEL => return Err(Error::Canceled),
                    ty => return Err(Error::BadPacketType(ty)),
                }
            }

            id = id.wrapping_add(1);
            print!("\x1B[0K\r[");
            let percent = i as f32 / chunk_count as f32 * 10.0;
            for i in 1..11 {
                if percent > i as f32 {
                    print!("=");
                } else {
                    print!(" ");
                }
            }
            print!("]");
            let _ = std::io::stdout().flush();
        }

        self.device.write(END_OF_TRANSMISSION)?;
        match self.device.read()? {
            ACK => {}
            ty => return Err(Error::BadPacketType(ty)),
        }

        Ok(())
    }
}

pub struct Receiver<S: SerialDevice> {
    buffer: Packet,
    device: S,
}

impl<S: SerialDevice> Receiver<S> {
    pub fn new(device: S) -> Self {
        Self {
            buffer: Packet::zeroed(),
            device,
        }
    }

    pub fn receive(&mut self, mut f: impl FnMut(&[u8; 128])) -> Result<(), Error<S::Error>> {
        self.device.write(CHECKSUM_REQUEST)?;
        let mut id = None;

        loop {
            let bytes = &mut bytemuck::bytes_of_mut(&mut self.buffer)[1..];
            match self.device.read()? {
                START_OF_HEADER => {}
                END_OF_TRANSMISSION => {
                    self.device.write(ACK)?;
                    break;
                }
                ty => return Err(Error::BadPacketType(ty)),
            }

            for byte in bytes {
                *byte = self.device.read()?;
            }

            let checksum_good = checksum(&self.buffer.data) == u16::from_be_bytes(self.buffer.crc);
            let id_good = self.buffer.id == !self.buffer.id_inverted;
            if !checksum_good || !id_good {
                self.device.write(NAK)?;
                continue;
            }

            match id {
                None => id = Some(self.buffer.id),
                Some(id) => {
                    if !(id == self.buffer.id + 1 || id == self.buffer.id - 1) {
                        return Err(Error::BadPacketId);
                    }
                }
            }

            f(&self.buffer.data);
        }

        match self.device.read()? {
            END_OF_TRANSMISSION_BLOCK => {}
            ty => return Err(Error::BadPacketType(ty)),
        }
        self.device.write(ACK)?;

        Ok(())
    }
}

pub fn checksum(bytes: &[u8]) -> u16 {
    let mut checksum = 0;

    for byte in bytes.iter().copied() {
        checksum ^= u16::from(byte) << 8;
        for _ in 0..8 {
            match (checksum as i16).is_negative() {
                true => checksum = (checksum << 1) ^ 0x1021,
                false => checksum <<= 1,
            }
        }
    }

    checksum
}
