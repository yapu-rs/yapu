#[allow(unused_imports)]
use binrw::{BinRead, BinWrite, binread, binrw, binwrite};
use std::ops::RangeInclusive;
use std::borrow::Cow;
use std::ops::{Deref, DerefMut};

/// Protocol conversion error
#[derive(Debug, Clone)]
pub enum Error {
    Exceeded(usize, RangeInclusive<usize>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exceeded(value, range) => {
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
    #[derive(Default, Debug, Clone)]
    pub struct Buffer {
        state: u8,
    }

    impl Buffer {
        pub fn new() -> Self { Self::default() }
        pub fn state(&self) -> u8 {
            self.state
        }
    }

    impl std::io::Write for Buffer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.state = self.state ^ iter(buf.iter().copied());
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[bw(big)]
pub struct Opcode(u8, #[bw(calc = checksum::single(self.0))] u8);

impl Opcode {
    pub const GET: Self = Self(0x00u8);
    pub const GET_VERSION: Self = Self(0x01u8);
    pub const GET_ID: Self = Self(0x02u8);
    pub const READ: Self = Self(0x11u8);
    pub const GO: Self = Self(0x21u8);
    pub const WRITE: Self = Self(0x31u8);
    pub const ERASE: Self = Self(0x43u8);
    pub const EXTENDED_ERASE: Self = Self(0x44u8);
    pub const WRITE_PROTECT: Self = Self(0x63u8);
    pub const WRITE_UNPROTECT: Self = Self(0x73u8);
    pub const READ_PROTECT: Self = Self(0x82u8);
    pub const READ_UNPROTECT: Self = Self(0x92u8);
    pub const GET_CHECKSUM: Self = Self(0xa1u8);
    pub const SPECIAL: Self = Self(0x50u8);
    pub const EXTENDED_SPECIAL: Self = Self(0x51u8);

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl std::fmt::Display for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &Self::GET => write!(f, "GET"),
            &Self::GET_VERSION => write!(f, "GET_VERSION"),
            &Self::GET_ID => write!(f, "GET_ID"),
            &Self::READ => write!(f, "READ"),
            &Self::GO => write!(f, "GO"),
            &Self::WRITE => write!(f, "WRITE"),
            &Self::ERASE => write!(f, "ERASE"),
            &Self::EXTENDED_ERASE => write!(f, "EXTENDED_ERASE"),
            &Self::WRITE_PROTECT => write!(f, "WRITE_PROTECT"),
            &Self::WRITE_UNPROTECT => write!(f, "WRITE_UNPROTECT"),
            &Self::READ_PROTECT => write!(f, "READ_PROTECT"),
            &Self::READ_UNPROTECT => write!(f, "READ_UNPROTECT"),
            &Self::GET_CHECKSUM => write!(f, "GET_CHECKSUM"),
            &Self::SPECIAL => write!(f, "SPECIAL"),
            &Self::EXTENDED_SPECIAL => write!(f, "EXTENDED_SPECIAL"),
            opcode => write!(f, "UNKNOWN ({:02x?})", opcode.as_u8()),
        }
    }
}

impl From<u8> for Opcode {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

/// Address
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

macro_rules! define_slice_item {
    ($vis:vis $name:ident($inner_ty:ident), $as_method:ident, $size_ty:ty, $size_range:expr) => {
        #[derive(BinWrite, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $name;

        impl SliceItem for $name {
            type Repr = $inner_ty;
            type Size = $size_ty;
            const SIZE_RANGE: RangeInclusive<usize> = $size_range;
        }
    }
}

pub type PageNo = u8;
pub type ExtendedPageNo = u16;
pub type SectorNo = u8;

define_slice_item! { pub Byte(u8), as_u8, u8, 1..=256 }
define_slice_item! { pub Page(PageNo), as_u8, u8, 1..=256 }
define_slice_item! { pub ExtendedPage(ExtendedPageNo), as_u16, u16, 1..=0xff00 }
define_slice_item! { pub Sector(SectorNo), as_u8, u8, 1..=256 }

#[binwrite]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[bw(big)]
pub struct Size(u8, #[bw(calc = checksum::single(self.0))] u8);

impl Into<usize> for Size {
    fn into(self) -> usize { self.0 as usize }
}

impl Into<u8> for Size {
    fn into(self) -> u8 { self.0 }
}

impl TryFrom<usize> for Size {
    type Error = Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let range = <Byte as SliceItem>::SIZE_RANGE;
        if range.contains(&value) {
            Ok(Self(value as u8))
        } else {
            Err(Error::Exceeded(value, range))
        }
    }
}

pub trait SliceItem {
    type Repr: Copy + Clone;
    type Size: TryFrom<usize>;
    const SIZE_RANGE: RangeInclusive<usize> = usize::MIN..=usize::MAX;
}

#[derive(Debug, Clone)]
pub struct Slice<'a, T: SliceItem> {
    inner: Cow<'a, [T::Repr]>,
}

impl<'a, T: SliceItem> Deref for Slice<'a, T> {
    type Target = Cow<'a, [T::Repr]>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T: SliceItem> DerefMut for Slice<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, T: SliceItem> Slice<'a, T> {
    /// Consumes [`Slice`] and returns the inner [`Cow`].
    pub fn into_inner(self) -> Cow<'a, [T::Repr]> {
        self.inner
    }

    /// Returns slice of specific slice items.
    pub fn as_slice(&self) -> &[T::Repr] {
        &self.inner
    }
}

impl<'a, T: SliceItem> Into<Cow<'a, [T::Repr]>> for Slice<'a, T> {
    fn into(self) -> Cow<'a, [T::Repr]> {
        self.inner
    }
}

impl<'a, T: SliceItem> TryFrom<Cow<'a, [T::Repr]>> for Slice<'a, T> {
    type Error = Error;

    fn try_from(value: Cow<'a, [T::Repr]>) -> Result<Self, Self::Error> {
        if T::SIZE_RANGE.contains(&value.len()) {
            Ok(Self { inner: value })
        } else {
            Err(Error::Exceeded(value.len(), T::SIZE_RANGE))
        }
    }
}

impl<'a, T: SliceItem> TryFrom<Vec<T::Repr>> for Slice<'a, T>
{
    type Error = Error;

    fn try_from(value: Vec<T::Repr>) -> Result<Self, Self::Error> {
        if T::SIZE_RANGE.contains(&value.len()) {
            Ok(Self { inner: value.into() })
        } else {
            Err(Error::Exceeded(value.len(), T::SIZE_RANGE))
        }
    }
}

impl<'a, T: SliceItem> TryFrom<&'a [T::Repr]> for Slice<'a, T> {
    type Error = Error;

    fn try_from(value: &'a [T::Repr]) -> Result<Self, Self::Error> {
        if T::SIZE_RANGE.contains(&value.len()) {
            Ok(Self { inner: value.into() })
        } else {
            Err(Error::Exceeded(value.len(), T::SIZE_RANGE))
        }
    }
}

impl<'a, T: SliceItem + BinWrite<Args<'a> = ()>> BinWrite for Slice<'a, T>
where
    [T::Repr]: BinWrite<Args<'a> = ()>,
    T::Size: BinWrite<Args<'a> = ()>,
    <T::Size as TryFrom<usize>>::Error: std::fmt::Debug
{
    type Args<'arg> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        use binrw::io::NoSeek;

        // write shifted size
        let lowerbound = *<T as SliceItem>::SIZE_RANGE.start();
        let size = <T as SliceItem>::Size::try_from(self.inner.len() - lowerbound).unwrap();
        size.write_options(writer, endian, args)?;

        // write data
        self.inner.write_options(writer, endian, args)?;

        // write checksum
        let mut buffer = checksum::Buffer::new();
        self.inner.write_options(&mut NoSeek::new(&mut buffer), endian, args)?;
        buffer.state().write_options(writer, endian, args)?;

        Ok(())
    }
}

impl<'a, T: SliceItem + BinWrite<Args<'a> = ()>> binrw::meta::WriteEndian for Slice<'a, T>
where
    [T::Repr]: BinWrite<Args<'a> = ()>,
    T::Size: BinWrite<Args<'a> = ()>,
    <T::Size as TryFrom<usize>>::Error: std::fmt::Debug
{
    const ENDIAN: binrw::meta::EndianKind = binrw::meta::EndianKind::Endian(binrw::Endian::Big);
}

pub type Data<'a> = Slice<'a, Byte>;
pub type PageNos<'a> = Slice<'a, Page>;
pub type ExtendedPageNos<'a> = Slice<'a, ExtendedPage>;
pub type SectorNos<'a> = Slice<'a, Sector>;

/// Command
#[binwrite]
#[derive(Debug, Clone)]
#[bw(big)]
pub enum Command<'a> {
    Get(#[bw(calc = Opcode::GET)] Opcode),
    Version(#[bw(calc = Opcode::GET_VERSION)] Opcode),
    Id(#[bw(calc = Opcode::GET_ID)] Opcode),
    Read {
        #[bw(calc = Opcode::READ)]
        opcode: Opcode,
        address: Address,
        size: Size,
    },
    Go(#[bw(calc = Opcode::GO)] Opcode, Address),
    Write {
        #[bw(calc = Opcode::WRITE)]
        opcode: Opcode,
        address: Address,
        data: Data<'a>,
    },
    Erase(#[bw(calc = Opcode::ERASE)] Opcode, Erase<'a>),
    ExtendedErase(#[bw(calc = Opcode::ERASE)] Opcode, ExtendedErase<'a>),
    WriteProtect(#[bw(calc = Opcode::WRITE_PROTECT)] Opcode),
    WriteUnprotect(#[bw(calc = Opcode::WRITE_UNPROTECT)] Opcode),
    ReadProtect(#[bw(calc = Opcode::READ_PROTECT)] Opcode),
    ReadUnprotect(#[bw(calc = Opcode::READ_UNPROTECT)] Opcode),

    /// This is used for baudrate handshaking.
    #[bw(magic = 0x7fu8)]
    Synchronize,
}

/// Command for [`Opcode::ERASE`].
#[derive(BinWrite, Debug, Clone)]
#[bw(big)]
pub enum Erase<'a> {
    #[bw(magic = 0xff00u16)]
    Global,
    Specific(Slice<'a, Page>),
}

impl<'a> Erase<'a> {
    /// Whether erasure is done globally.
    pub fn is_global(self) -> bool {
        matches!(self, Self::Global)
    }

    /// Whether erasure is done on specific pages.
    pub fn is_specific(self) -> bool {
        matches!(self, Self::Specific(..))
    }

    /// Returns pages if the erasure is not global.
    pub fn pages(&self) -> Option<&[PageNo]> {
        match self {
            Self::Global => None,
            Self::Specific(slice) => Some(slice.as_slice()),
        }
    }
}

/// Command for [`Opcode::EXTENDED_ERASE`].
#[derive(BinWrite, Debug, Clone)]
#[bw(big)]
pub enum ExtendedErase<'a> {
    #[bw(magic = b"\xff\xff\x00")]
    Global,
    #[bw(magic = b"\xff\xfe\x01")]
    Bank1,
    #[bw(magic = b"\xff\xfd\x02")]
    Bank2,
    Specific(Slice<'a, ExtendedPage>),
}

impl<'a> ExtendedErase<'a> {
    /// Whether erasure is done globally.
    pub fn is_global(self) -> bool {
        matches!(self, Self::Global)
    }

    /// Whether erasure is done on bank 1.
    pub fn is_bank1(self) -> bool {
        matches!(self, Self::Bank1)
    }

    /// Whether erasure is done on bank 2.
    pub fn is_bank2(self) -> bool {
        matches!(self, Self::Bank2)
    }
 
    /// Whether erasure is done on specific pages.
    pub fn is_specific(self) -> bool {
        matches!(self, Self::Specific(..))
    }

    /// Returns pages if the erasure is not global.
    pub fn pages(&self) -> Option<&[ExtendedPageNo]> {
        match self {
            Self::Global => None,
            Self::Bank1 => None,
            Self::Bank2 => None,
            Self::Specific(slice) => Some(slice.as_slice()),
        }
    }
}

/// Reply
#[derive(BinRead, Debug, Clone, Copy)]
#[br(big)]
pub enum Reply {
    /// ACK
    #[brw(magic = 0x79u8)]
    Ack,
    /// Negative
    #[brw(magic = 0x1fu8)]
    NAck,
}

/// Bootloader information
///
/// Contains version and supported [`Opcode`]s.
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
    /// Bootloader version in [`u8`].
    #[inline]
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Bootloader major version.
    #[inline]
    pub fn major(&self) -> u8 {
        self.version >> 4
    }

    /// Bootloader minor version.
    #[inline]
    pub fn minor(&self) -> u8 {
        self.version & 0xf
    }

    /// Bootloader version string.
    pub fn version_string(&self) -> String {
        format!("{}.{}", self.major(), self.minor())
    }

    /// Supported [`Opcode`]s of the bootloader.
    #[inline]
    pub fn opcodes(&self) -> &[Opcode] {
        &self.opcodes
    }

    /// Whether bootloader supports an [`Opcode`].
    #[inline]
    pub fn supports(&self, opcode: impl Into<Opcode>) -> bool {
        self.opcodes.contains(&opcode.into())
    }
}

/// Version
#[derive(BinRead, Debug, Clone)]
#[br(big)]
pub struct Version {
    version: u8,
    options: [u8; 2],
}

impl Version {
    /// Bootloader version in [`u8`].
    #[inline]
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Bootloader major version.
    #[inline]
    pub fn major(&self) -> u8 {
        self.version >> 4
    }

    /// Bootloader minor version.
    #[inline]
    pub fn minor(&self) -> u8 {
        self.version & 0xf
    }

    /// Bootloader version string.
    pub fn version_string(&self) -> String {
        format!("{}.{}", self.major(), self.minor())
    }

    #[inline]
    pub fn options(&self) -> [u8; 2] {
        self.options
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major(), self.minor())
    }
}

/// Chip ID
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
    /// Consumes [`Id`] and returns raw chip ID in [`Vec<u8>`].
    #[inline]
    pub fn into_id(self) -> Vec<u8> {
        self.id
    }

    /// Returns raw chip ID in [`[u8]`] slice.
    #[inline]
    pub fn id(&self) -> &[u8] {
        &self.id
    }

    /// Returns raw chip ID in [`[u8]`] slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.id
    }

    /// Converts chip ID to a fixed-size array.
    pub fn as_array<const N: usize>(&self) -> [u8; N] {
        let mut buf: [u8; N] = [0u8; N];
        buf[N - self.id.len()..].copy_from_slice(&self.id);
        buf
    }

    /// Interprets chip ID as [`u16`].
    #[inline]
    pub fn as_u16(&self) -> u16 {
        u16::from_be_bytes(self.as_array())
    }

    /// Interprets chip ID as [`u32`].
    #[inline]
    pub fn as_u32(&self) -> u32 {
        u32::from_be_bytes(self.as_array())
    }

    /// Interprets chip ID as [`u64`].
    #[inline]
    pub fn as_u64(&self) -> u64 {
        u64::from_be_bytes(self.as_array())
    }
}
