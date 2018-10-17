// Copyright 2018 Ed McCardell
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg_attr(feature = "cargo-clippy", feature(tool_lints))]

use std::fmt;

use machine_int::MachineInt;

use self::mi::Byte;

pub use crate::cmos::Cmos;
pub use crate::nmos::Nmos;

mod cmos;
mod mi;
mod nmos;

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub enum NmiLength {
    One,
    Two,
    Plenty,
}

pub trait Sys {
    fn read(&mut self, addr: u16) -> Option<u8>;

    fn write(&mut self, addr: u16, val: u8) -> Option<()>;

    #[inline]
    fn set_sync(&mut self, _set: bool) {}

    #[inline]
    fn poll_nmi(&mut self) -> bool {
        false
    }

    #[inline]
    fn peek_nmi(&self) -> bool {
        false
    }

    #[inline]
    fn nmi_length(&self) -> NmiLength {
        NmiLength::Plenty
    }

    #[inline]
    fn irq(&self) -> bool {
        false
    }
}

pub trait Cpu: Clone + fmt::Debug {
    fn is_nmos(&self) -> bool;
    fn reset(&mut self);
    fn pc(&self) -> u16;
    fn set_pc(&mut self, val: u16);
    fn sp(&self) -> u8;
    fn set_sp(&mut self, val: u8);
    fn a(&self) -> u8;
    fn set_a(&mut self, val: u8);
    fn x(&self) -> u8;
    fn set_x(&mut self, val: u8);
    fn y(&self) -> u8;
    fn set_y(&mut self, val: u8);
    fn status(&self) -> u8;
    fn set_status(&mut self, val: u8);
    fn flag(&self, f: Status) -> bool;
    fn set_flag(&mut self, f: Status, set: bool);
    fn run_instruction<S: Sys>(&mut self, sys: &mut S) -> Option<()>;
    fn partial_inst(&self) -> bool;
    fn halted(&self) -> bool;
}

#[derive(Copy, Clone)]
pub enum Status {
    N,
    V,
    D,
    I,
    Z,
    C,
}

#[derive(Clone, Default)]
struct Flags {
    n: Byte,
    v: Byte,
    _r: u8,
    _b: u8,
    d: bool,
    i: bool,
    z: Byte,
    c: Byte,
}

impl Flags {
    #[inline]
    fn n(&self) -> bool {
        (self.n & 0x80) != 0
    }

    #[inline]
    fn set_n(&mut self, set: bool) {
        self.n = MachineInt((set as u8) << 7)
    }

    #[inline]
    fn v(&self) -> bool {
        self.v != 0
    }

    #[inline]
    fn set_v(&mut self, set: bool) {
        self.v = MachineInt(set as u8);
    }

    #[inline]
    fn d(&self) -> bool {
        self.d
    }

    #[inline]
    fn set_d(&mut self, set: bool) {
        self.d = set
    }

    #[inline]
    fn i(&self) -> bool {
        self.i
    }

    #[inline]
    fn set_i(&mut self, set: bool) {
        self.i = set
    }

    #[inline]
    fn z(&self) -> bool {
        self.z == 0
    }

    #[inline]
    fn set_z(&mut self, set: bool) {
        self.z = MachineInt(!set as u8);
    }

    #[inline]
    pub fn c(&self) -> bool {
        self.c != 0
    }

    #[inline]
    pub fn set_c(&mut self, set: bool) {
        self.c = MachineInt(set as u8);
    }

    #[inline]
    fn nz(&mut self, val: Byte) {
        self.n = val;
        self.z = val;
    }

    fn to_byte(&self) -> Byte {
        self.n & 0x80
            | (self.v() as u8) << 6
            | 0x30
            | (self.d as u8) << 3
            | (self.i as u8) << 2
            | (self.z() as u8) << 1
            | self.c
    }

    #[cfg_attr(
        feature = "cargo-clippy",
        allow(clippy::wrong_self_convention)
    )]
    fn from_byte(&mut self, val: Byte) {
        self.n = val;
        self.v = val & 0x40;
        self.d = (val & 0x08) != 0;
        self.i = (val & 0x04) != 0;
        self.set_z((val & 0x02) != 0);
        self.c = val & 1;
    }
}

impl fmt::Debug for Flags {
    #[cfg_attr(
        feature = "cargo-clippy",
        allow(clippy::many_single_char_names)
    )]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let n = if self.n() { "N" } else { "n" };
        let v = match self.v.0 {
            0 => "v",
            _ => "V",
        };
        let d = if self.d { "D" } else { "d" };
        let i = if !self.i { "i" } else { "I" };
        let z = match self.z.0 {
            0 => "Z",
            _ => "z",
        };
        let c = match self.c.0 {
            0 => "c",
            _ => "C",
        };
        write!(f, "{}{}-B{}{}{}{}", n, v, d, i, z, c)
    }
}

#[cfg(test)]
mod test;
