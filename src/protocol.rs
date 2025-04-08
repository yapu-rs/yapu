#[allow(unused_imports)]
use binrw::{BinRead, BinWrite, binread, binrw, binwrite};
use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub enum Error {
    SizeExceeded(usize, RangeInclusive<usize>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SizeExceeded(value, range) => {
                write!(
                    f,
                    "{} is not within valid range of size ({:?})",
                    value, range
                )
            }
        }
    }
}

impl std::error::Error for Error {}

mod checksum {
    pub(super) fn single(data: u8) -> u8 {
        data ^ 0xff
    }

    pub(super) fn iter(data: impl Iterator<Item = u8>) -> u8 {
        data.fold(0u8, |acc, e| acc ^ e)
    }
}

/// A wrapper type for opcode.
///
/// `binrw` only supports magic literals, which means any computed value is not
/// supported, no matter it's constant or not. Therefore it's not possible to
/// write:
///
/// ```rust
/// #[derive(BinWrite)]
/// #[bw(big)]
/// enum Command {
///     #[bw(magic = (0x00u8 << 8) ^ (0x00u8 ^ 0xffu8))]
///     Get,
/// }
/// ```
///
/// The workaround here is to define a new wrapper type for opcodes and add a
/// checksum field with computed `binrw` values, which requires using procedural
/// macro `binwrite` rather than derive macro `BinWrite`.
#[binwrite]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[bw(big)]
pub struct Opcode(u8, #[bw(calc = checksum::single(self.0))] u8);

impl Opcode {
    pub const GET: Self = Self(0x00u8);
    pub const VERSION: Self = Self(0x01u8);
    pub const ID: Self = Self(0x02u8);
    pub const READ: Self = Self(0x11u8);
    pub const WRITE: Self = Self(0x31u8);
    pub const ERASE: Self = Self(0x44u8);
    pub const GO: Self = Self(0x21u8);
    pub const WRITE_LOCK: Self = Self(0x63u8);
    pub const WRITE_UNLOCK: Self = Self(0x73u8);
    pub const READ_LOCK: Self = Self(0x82u8);
    pub const READ_UNLOCK: Self = Self(0x92u8);

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl std::fmt::Display for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &Self::GET => write!(f, "GET"),
            &Self::VERSION => write!(f, "VERSION"),
            &Self::ID => write!(f, "ID"),
            &Self::READ => write!(f, "READ"),
            &Self::WRITE => write!(f, "WRITE"),
            &Self::ERASE => write!(f, "ERASE"),
            &Self::GO => write!(f, "GO"),
            &Self::WRITE_LOCK => write!(f, "WRITE_LOCK"),
            &Self::WRITE_UNLOCK => write!(f, "READ_LOCK"),
            &Self::READ_LOCK => write!(f, "READ_LOCK"),
            &Self::READ_UNLOCK => write!(f, "READ_UNLOCK"),
            opcode => write!(f, "UNKNOWN ({:02x?})", opcode.as_u8()),
        }
    }
}

impl From<u8> for Opcode {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

#[binwrite]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[bw(big)]
pub struct Address(
    u32,
    #[bw(calc = checksum::iter(self.0.to_ne_bytes().iter().copied()))] u8,
);

impl Address {
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl From<u32> for Address {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Into<u32> for Address {
    fn into(self) -> u32 {
        self.0
    }
}

#[binwrite]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[bw(big)]
pub struct Size(u8, #[bw(calc = checksum::single(self.0))] u8);

impl Size {
    pub fn as_usize(&self) -> usize {
        self.0 as usize + 1usize
    }
}

impl Into<usize> for Size {
    fn into(self) -> usize {
        self.0 as usize + 1usize
    }
}

impl From<u8> for Size {
    fn from(value: u8) -> Self {
        Self(value - 1u8)
    }
}

impl TryFrom<usize> for Size {
    type Error = Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let range = 1..=256;
        if range.contains(&value) {
            Ok(Self((value as u8) - 1u8))
        } else {
            Err(Error::SizeExceeded(value, range))
        }
    }
}

#[binwrite]
#[derive(Debug, Clone)]
#[bw(big)]
pub struct Data<'a> {
    #[bw(calc = (data.len() - 1) as u8)]
    length: u8,
    pub(crate) data: &'a [u8],
    #[bw(calc = checksum::iter(data.iter().copied()))]
    checksum: u8,
}

#[binwrite]
#[derive(Debug, Clone)]
#[bw(big)]
pub enum Command<'a> {
    Get(#[bw(calc = Opcode::GET)] Opcode),
    Version(#[bw(calc = Opcode::VERSION)] Opcode),
    Id(#[bw(calc = Opcode::ID)] Opcode),
    Read {
        #[bw(calc = Opcode::READ)]
        opcode: Opcode,
        address: Address,
        size: Size,
    },
    Write {
        #[bw(calc = Opcode::WRITE)]
        opcode: Opcode,
        address: Address,
        data: Data<'a>,
    },
    Erase {
        #[bw(calc = Opcode::ERASE)]
        opcode: Opcode,
        address: Address,
    },
    Go(#[bw(calc = Opcode::GO)] Opcode, Address),
    WriteLock(#[bw(calc = Opcode::WRITE_LOCK)] Opcode),
    WriteUnlock(#[bw(calc = Opcode::WRITE_UNLOCK)] Opcode),
    ReadLock(#[bw(calc = Opcode::READ_LOCK)] Opcode),
    ReadUnlock(#[bw(calc = Opcode::READ_UNLOCK)] Opcode),

    /// This is used for baudrate handshaking.
    #[bw(magic = 0x7fu8)]
    Synchronize,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(big)]
pub enum Reply {
    #[brw(magic = 0x79u8)]
    Ack,
    #[brw(magic = 0x1fu8)]
    NAck,
    #[brw(magic = 0xaau8)]
    Busy,
}

#[binread]
#[derive(Debug, Clone)]
#[br(big)]
pub struct Bootloader {
    #[br(temp)]
    len: u8,
    version: u8,
    #[br(count = len, map = |data: Vec<u8>| {
        data.into_iter().map(|v| v.into()).collect()
    })]
    opcodes: Vec<Opcode>,
}

impl Bootloader {
    pub fn version(&self) -> u8 {
        self.version
    }
    pub fn major(&self) -> u8 {
        self.version >> 4
    }
    pub fn minor(&self) -> u8 {
        self.version & 0xf
    }
    pub fn version_string(&self) -> String {
        format!("{}.{}", self.major(), self.minor())
    }
    pub fn opcodes(&self) -> &[Opcode] {
        &self.opcodes
    }
}

#[derive(BinRead, Debug, Clone)]
#[br(big)]
pub struct Version {
    version: u8,
    options: [u8; 2],
}

impl Version {
    pub fn version(&self) -> u8 {
        self.version
    }
    pub fn options(&self) -> [u8; 2] {
        self.options
    }
}

#[binread]
#[derive(Debug, Clone)]
#[br(big)]
pub struct Id {
    #[br(temp)]
    len: u8,
    #[br(count = len + 1)]
    id: Vec<u8>,
}

impl Id {
    pub fn into_id(self) -> Vec<u8> {
        self.id
    }
    pub fn id(&self) -> &[u8] {
        &self.id
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.id
    }
    pub fn as_array<const N: usize>(&self) -> [u8; N] {
        let mut buf: [u8; N] = [0u8; N];
        buf[N - self.id.len()..].copy_from_slice(&self.id);
        buf
    }
    pub fn as_u16(&self) -> u16 {
        u16::from_be_bytes(self.as_array())
    }
    pub fn as_u32(&self) -> u32 {
        u32::from_be_bytes(self.as_array())
    }
    pub fn as_u64(&self) -> u64 {
        u64::from_be_bytes(self.as_array())
    }
}
