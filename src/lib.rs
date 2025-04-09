mod probe;
mod protocol;

pub use probe::{Probe, ProbeBuilder, Signal, SignalScheme, SignalSchemeBuilder};
use protocol::{Bootloader, Data, Id, Version};
use protocol::{Command, Opcode, Reply};

use binrw::io::NoSeek;
use binrw::meta::{ReadEndian, WriteEndian};
use binrw::{BinRead, BinWrite};
use log::trace;
use serialport::ClearBuffer;
use serialport::SerialPort;
use serialport::{DataBits, FlowControl, Parity, StopBits};
use std::borrow::Cow;

/// Error
#[derive(Debug)]
pub enum Error {
    NAck,
    Busy,
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
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Busy)
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
            Self::NAck => write!(f, "nack error"),
            Self::Busy => write!(f, "target is busy"),
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

/// Programmer
#[derive(Debug)]
pub struct Programmer {
    port: Box<dyn SerialPort>,
    probe: Probe,
}

impl Programmer {
    /// Read all contents from the device.
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

    pub fn port<'a>(path: impl Into<Cow<'a, str>>, probe: &Probe) -> Result<Box<dyn SerialPort>> {
        let port = serialport::new(path, probe.baudrate())
            .data_bits(DataBits::Eight)
            .parity(Parity::Even)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(probe.timeout())
            .open()?;
        Ok(port)
    }

    /// Create a programmer from an existing serial port without handshaking
    pub fn attach(port: Box<dyn SerialPort>, probe: &Probe) -> Self {
        Self {
            port,
            probe: probe.clone(),
        }
    }

    pub fn open<'a>(path: impl Into<Cow<'a, str>>, probe: &Probe) -> Result<Self> {
        let port = Self::port(path, probe)?;
        let mut programmer = Self {
            port,
            probe: probe.clone(),
        };
        programmer.identify()?;
        Ok(programmer)
    }

    /// Send data
    ///
    /// This sends data to the port.
    pub fn send<T: for<'b> BinWrite<Args<'b> = ()> + WriteEndian>(
        &mut self,
        data: T,
    ) -> Result<()> {
        let mut wrapper = NoSeek::new(&mut self.port);
        data.write(&mut wrapper)?;
        Ok(())
    }

    /// Send data through reliable channels
    ///
    /// This sends data to the port and expects a reply from the controller.
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
            Reply::Busy => Err(Error::Busy),
            Reply::Ack => Ok(()),
        }
    }

    pub fn recv<T: for<'b> BinRead<Args<'b> = ()> + ReadEndian>(&mut self) -> Result<T> {
        let mut wrapper = NoSeek::new(&mut self.port);
        let data = T::read(&mut wrapper)?;
        Ok(data)
    }

    pub fn recv_reliable<T: for<'b> BinRead<Args<'b> = ()> + ReadEndian>(&mut self) -> Result<T> {
        let mut wrapper = NoSeek::new(&mut self.port);
        let data = T::read(&mut wrapper)?;
        self.send(())?;
        Ok(data)
    }

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
            other => self.send_reliable(other),
        }
    }

    pub fn set_signal(&mut self, signal: Signal, active: bool) -> Result<()> {
        let raw = signal.raw_level(active);
        match signal {
            Signal::Rts { .. } => self.port.write_request_to_send(raw)?,
            Signal::Dtr { .. } => self.port.write_data_terminal_ready(raw)?,
        }
        Ok(())
    }

    pub fn set_boot(&mut self, active: bool) -> Result<()> {
        if let Some(signal) = self.probe.signal_boot() {
            self.set_signal(signal, active)?;
        }
        Ok(())
    }

    pub fn set_reset(&mut self, active: bool) -> Result<()> {
        if let Some(signal) = self.probe.signal_reset() {
            self.set_signal(signal, active)?;
        }
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
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
        self.port.clear(ClearBuffer::All)?;
        self.set_boot(true)?;
        while retries < self.probe.max_attempts() {
            self.reset()?;
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

    pub fn discover(probe: &Probe) -> Result<Vec<Self>> {
        let ports = serialport::available_ports()?
            .into_iter()
            .filter_map(|s| Self::open(s.port_name, probe).ok())
            .collect();
        Ok(ports)
    }

    pub fn read_bootloader(&mut self) -> Result<Bootloader> {
        self.send_command(Command::Get())?;
        let bootloader: Bootloader = self.recv_reliable()?;
        Ok(bootloader)
    }

    pub fn read_version(&mut self) -> Result<Version> {
        self.send_command(Command::Version())?;
        let version: Version = self.recv_reliable()?;
        Ok(version)
    }

    pub fn read_id(&mut self) -> Result<Id> {
        self.send_command(Command::Id())?;
        let id: Id = self.recv_reliable()?;
        Ok(id)
    }

    pub fn read_memory(&mut self, address: u32, size: usize) -> Result<Vec<u8>> {
        self.send_command(Command::Read {
            address: address.into(),
            size: size.try_into()?,
        })?;
        let mut data = vec![0u8; size];
        self.port.read_exact(&mut data)?;
        Ok(data)
    }

    pub fn write_memory<'a>(&mut self, address: u32, data: &'a [u8]) -> Result<()> {
        let data = Data { data };
        self.send_reliable(Command::Write {
            address: address.into(),
            data,
        })?;
        Ok(())
    }

    pub fn inner(&self) -> &Box<dyn SerialPort> {
        &self.port
    }

    pub fn into_inner(self) -> Box<dyn SerialPort> {
        self.port
    }
}
