//! # YAPU: Yet Another Programmer via USART
//!
//! AN3155-compliant programmer
//!
//! [![license][license badge]][repo]
//! [![crates.io version][crates.io version badge]][crate]
//!
//! The library implements the protocols used by AN3155-compliant bootloaders
//! and offers device discovery. Check [repo README][repo] for more details.
//!
//! A binary `yapu` is also shipped in the [crate][crate] for common use.
//!
//! [repo]: https://github.com/yapu-rs/yapu
//! [crate]: https://crates.io/crates/yapu
//!
//! [license badge]: https://img.shields.io/github/license/yapu-rs/yapu?style=flat
//! [crates.io version badge]: https://img.shields.io/crates/v/yapu?style=flat

mod probe;
mod protocol;

pub use probe::{Probe, ProbeBuilder, Signal, SignalScheme, SignalSchemeBuilder};

// Common requests and responses in the protocol
pub use protocol::{Command, Opcode, Reply, Address};
pub use protocol::{Erase, ExtendedErase};
pub use protocol::{Bootloader, Id, Version};

// Slice and slice items defined in the protocol
pub use protocol::{Slice, SliceItem};
pub use protocol::{Byte, Data, PageNo, PageNos, ExtendedPageNo, ExtendedPageNos, SectorNo, SectorNos};

use binrw::io::NoSeek;
use binrw::meta::{ReadEndian, WriteEndian};
use binrw::{BinRead, BinWrite};
use log::trace;
use serialport::ClearBuffer;
use serialport::SerialPort;
use serialport::{DataBits, FlowControl, Parity, StopBits};

/// Error
#[derive(Debug)]
pub enum Error {
    NAck,
    Unidentified,
    ProtocolConversion(protocol::Error),
    Io(std::io::Error),
    Serial(serialport::Error),
    Frame(binrw::Error),
}

impl Error {
    pub fn is_nack(&self) -> bool {
        matches!(self, Self::NAck)
    }
    pub fn is_unidentified(&self) -> bool {
        matches!(self, Self::Unidentified)
    }

    pub fn is_protocol_conversion(&self) -> bool {
        matches!(self, Self::ProtocolConversion(..))
    }
    pub fn as_protocol_conversion(&self) -> Option<&protocol::Error> {
        match self {
            Self::ProtocolConversion(e) => Some(e),
            _ => None,
        }
    }
    pub fn into_protocol_conversion(self) -> Option<protocol::Error> {
        match self {
            Self::ProtocolConversion(e) => Some(e),
            _ => None,
        }
    }

    pub fn is_io_error(&self) -> bool {
        matches!(self, Self::Io(..))
    }
    pub fn as_io_error(&self) -> Option<&std::io::Error> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
    pub fn into_io_error(self) -> Option<std::io::Error> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }

    pub fn is_serial_error(&self) -> bool {
        matches!(self, Self::Serial(..))
    }
    pub fn as_serial_error(&self) -> Option<&serialport::Error> {
        match self {
            Self::Serial(e) => Some(e),
            _ => None,
        }
    }
    pub fn into_serial_error(self) -> Option<serialport::Error> {
        match self {
            Self::Serial(e) => Some(e),
            _ => None,
        }
    }

    pub fn is_frame_error(&self) -> bool {
        matches!(self, Self::Frame(..))
    }
    pub fn as_frame_error(&self) -> Option<&binrw::Error> {
        match self {
            Self::Frame(e) => Some(e),
            _ => None,
        }
    }
    pub fn into_frame_error(self) -> Option<binrw::Error> {
        match self {
            Self::Frame(e) => Some(e),
            _ => None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NAck => write!(f, "negative ack"),
            Self::Unidentified => write!(f, "cannot identify device"),
            Self::ProtocolConversion(e) => write!(f, "protocol conversion error: {}", e),
            Self::Io(e) => write!(f, "io error: {}", e),
            Self::Serial(e) => write!(f, "serial error: {}", e),
            Self::Frame(e) => write!(f, "frame error: {}", e),
        }
    }
}

impl From<protocol::Error> for Error {
    fn from(value: protocol::Error) -> Self {
        Self::ProtocolConversion(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serialport::Error> for Error {
    fn from(value: serialport::Error) -> Self {
        Self::Serial(value)
    }
}

impl From<binrw::Error> for Error {
    fn from(value: binrw::Error) -> Self {
        Self::Frame(value)
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

/// AN3155-compliant programmer
#[derive(Debug)]
pub struct Programmer {
    port: Box<dyn SerialPort>,
    probe: Probe,
}

impl Programmer {
    /// Reads all contents from the device.
    ///
    /// Not recommended to use.
    pub fn read_all(&mut self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let result = self.port.read_to_end(&mut buf);
        match result {
            Ok(_) => Ok(buf),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(buf),
            Err(e) => Err(e.into()),
        }
    }

    /// Opens a serial port by its name and configures it according to a probe.
    pub fn port(path: impl AsRef<str>, probe: &Probe) -> Result<Box<dyn SerialPort>> {
        let port = serialport::new(path.as_ref(), probe.baudrate())
            .data_bits(DataBits::Eight)
            .parity(Parity::Even)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(probe.timeout())
            .open()?;
        Ok(port)
    }

    /// Creates a programmer from an existing serial port without handshaking.
    pub fn attach(port: Box<dyn SerialPort>, probe: &Probe) -> Self {
        Self {
            port,
            probe: probe.clone(),
        }
    }

    /// Creates a programmer from a port name.
    pub fn open(path: impl AsRef<str>, probe: &Probe) -> Result<Self> {
        let port = Self::port(path.as_ref(), probe)?;
        let mut programmer = Self {
            port,
            probe: probe.clone(),
        };
        programmer.identify()?;
        Ok(programmer)
    }

    /// Sends serializable [`BinWrite`] data to the underlying port.
    pub fn send<T: for<'b> BinWrite<Args<'b> = ()> + WriteEndian>(
        &mut self,
        data: T,
    ) -> Result<()> {
        let mut wrapper = NoSeek::new(&mut self.port);
        data.write(&mut wrapper)?;
        Ok(())
    }

    /// Sends serializable [`BinWrite`] data through reliable channels.
    ///
    /// Unlike [`Self::send`], the sender expects a reply from the controller.
    pub fn send_reliable<T: for<'b> BinWrite<Args<'b> = ()> + WriteEndian>(
        &mut self,
        data: T,
    ) -> Result<()> {
        let mut wrapper = NoSeek::new(&mut self.port);
        data.write(&mut wrapper)?;
        let reply: Reply = Reply::read(&mut wrapper)?;
        trace!("received reliable reply: {:?}", reply);
        match reply {
            Reply::NAck => Err(Error::NAck),
            Reply::Ack => Ok(()),
        }
    }

    /// Receives serializable [`BinRead`] data from the underlying port.
    pub fn recv<T: for<'b> BinRead<Args<'b> = ()> + ReadEndian>(&mut self) -> Result<T> {
        let mut wrapper = NoSeek::new(&mut self.port);
        let data = T::read(&mut wrapper)?;
        Ok(data)
    }

    /// Receives serializable [`BinRead`] data through reliable channels.
    pub fn recv_reliable<T: for<'b> BinRead<Args<'b> = ()> + ReadEndian>(&mut self) -> Result<T> {
        let mut wrapper = NoSeek::new(&mut self.port);
        let data = T::read(&mut wrapper)?;
        self.send(())?;
        Ok(data)
    }

    /// Sends a [`Command`] defined in the protocol.
    pub fn send_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Read { address, size } => {
                self.send_reliable(Opcode::READ)?;
                self.send_reliable(address)?;
                self.send_reliable(size)
            }
            Command::Write { address, data } => {
                self.send_reliable(Opcode::WRITE)?;
                self.send_reliable(address)?;
                self.send_reliable(data)
            }
            Command::Erase(erase) => {
                self.send_reliable(Opcode::ERASE)?;
                self.send_reliable(erase)
            }
            Command::ExtendedErase(erase) => {
                self.send_reliable(Opcode::EXTENDED_ERASE)?;
                self.send_reliable(erase)
            }
            other => self.send_reliable(other),
        }
    }

    /// Changes a signal value of the underlying port.
    pub fn set_signal(&mut self, signal: Signal, active: bool) -> Result<()> {
        let raw = signal.raw_level(active);
        match signal {
            Signal::Rts { .. } => self.port.write_request_to_send(raw)?,
            Signal::Dtr { .. } => self.port.write_data_terminal_ready(raw)?,
        }
        Ok(())
    }

    /// Changes boot signal value of the underlying port.
    pub fn set_boot(&mut self, active: bool) -> Result<()> {
        if let Some(signal) = self.probe.signal_boot() {
            self.set_signal(signal, active)?;
        }
        Ok(())
    }

    /// Changes reset signal value of the underlying port.
    pub fn set_reset(&mut self, active: bool) -> Result<()> {
        if let Some(signal) = self.probe.signal_reset() {
            self.set_signal(signal, active)?;
        }
        Ok(())
    }

    /// Resets the device.
    pub fn reset(&mut self) -> Result<()> {
        if self.probe.signal_reset().is_some() {
            self.set_reset(false)?;
            self.set_reset(true)?;
            std::thread::sleep(self.probe.reset_for());
            self.set_reset(false)?;
        }
        Ok(())
    }

    fn identify(&mut self) -> Result<()> {
        let mut retries = 0;
        self.set_boot(true)?;
        while retries < self.probe.max_attempts() {
            self.reset()?;
            self.port.clear(ClearBuffer::All)?;
            match self.send_reliable(Command::Synchronize) {
                Ok(_) => {
                    self.set_boot(false)?;
                    self.port.clear(ClearBuffer::All)?;
                    return Ok(());
                }
                _ => {}
            }
            retries += 1;
        }
        Err(Error::Unidentified)
    }

    /// Discovers compliant devices using a probe.
    pub fn discover(probe: &Probe) -> Result<Vec<Self>> {
        let ports = serialport::available_ports()?
            .into_iter()
            .filter_map(|s| Self::open(s.port_name, probe).ok())
            .collect();
        Ok(ports)
    }

    /// Reads bootloader information.
    pub fn read_bootloader(&mut self) -> Result<Bootloader> {
        self.send_command(Command::Get())?;
        let bootloader: Bootloader = self.recv_reliable()?;
        Ok(bootloader)
    }

    /// Reads version.
    pub fn read_version(&mut self) -> Result<Version> {
        self.send_command(Command::Version())?;
        let version: Version = self.recv_reliable()?;
        Ok(version)
    }

    /// Reads chip ID.
    pub fn read_id(&mut self) -> Result<Id> {
        self.send_command(Command::Id())?;
        let id: Id = self.recv_reliable()?;
        Ok(id)
    }

    /// Reads memory at specific region.
    pub fn read_memory(&mut self, address: u32, size: usize) -> Result<Data> {
        self.send_command(Command::Read {
            address: address.into(),
            size: size.try_into()?,
        })?;
        let mut data = vec![0u8; size];
        self.port.read_exact(&mut data)?;
        Ok(data.try_into().unwrap())
    }

    /// Writes memory at specific region.
    pub fn write_memory<'a>(&mut self, address: u32, data: Data) -> Result<()> {
        self.send_reliable(Command::Write {
            address: address.into(),
            data,
        })?;
        Ok(())
    }

    /// Gets the underlying serial port.
    pub fn inner(&self) -> &Box<dyn SerialPort> {
        &self.port
    }

    /// Gets the underlying serial port and drops the programmer.
    pub fn into_inner(self) -> Box<dyn SerialPort> {
        self.port
    }
}
