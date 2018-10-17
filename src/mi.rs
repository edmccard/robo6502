pub use machine_int::{AsFrom, MachineInt};

pub trait AddrMath<T> {
    fn check_carry(self, offset: T) -> bool;
    fn no_carry(self, offset: T) -> Self;
}

impl AddrMath<u16> for MachineInt<u16> {
    #[inline]
    fn check_carry(self, offset: u16) -> bool {
        ((self & 0xff) + offset) > 0xff
    }

    #[inline]
    fn no_carry(self, offset: u16) -> Self {
        let lo_byte = ((self & 0xff) + offset) & 0xff;
        self & 0xff00 | lo_byte
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]
impl AddrMath<MachineInt<u8>> for MachineInt<u16> {
    #[inline]
    fn check_carry(self, offset: MachineInt<u8>) -> bool {
        self.check_carry(offset.0 as u16)
    }

    #[inline]
    fn no_carry(self, offset: MachineInt<u8>) -> Self {
        self.no_carry(offset.0 as u16)
    }
}

impl AddrMath<MachineInt<i8>> for MachineInt<u16> {
    #[inline]
    fn check_carry(self, offset: MachineInt<i8>) -> bool {
        ((self & 0xff) + offset) > 0xff
    }

    #[inline]
    fn no_carry(self, offset: MachineInt<i8>) -> Self {
        let lo_byte = ((self & 0xff) + offset) & 0xff;
        self & 0xff00 | lo_byte
    }
}

pub trait AddrExt {
    fn from_bytes(lo: Byte, hi: Byte) -> Self;
    fn zp(lo: Byte) -> Self;
    fn stack(lo: Byte) -> Self;
    fn hi(self) -> Byte;
    fn lo(self) -> Byte;
}

impl AddrExt for MachineInt<u16> {
    #[inline]
    fn from_bytes(lo: Byte, hi: Byte) -> Self {
        (Word::from(hi) << 8) | lo
    }

    #[inline]
    fn hi(self) -> Byte {
        Byte::as_from(self >> 8)
    }

    #[inline]
    fn lo(self) -> Byte {
        Byte::as_from(self)
    }

    #[inline]
    fn zp(lo: Byte) -> Self {
        Addr::from(lo)
    }

    #[inline]
    fn stack(lo: Byte) -> Self {
        Addr::from(lo) | 0x0100
    }
}

pub type Addr = MachineInt<u16>;
pub type SignedWord = MachineInt<i16>;
pub type Word = MachineInt<u16>;
pub type Byte = MachineInt<u8>;
pub type BranchOffset = MachineInt<i8>;
