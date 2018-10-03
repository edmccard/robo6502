// Copyright 2018 Ed McCardell
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg_attr(feature = "cargo-clippy", feature(tool_lints))]

mod core;

pub use crate::core::{Cpu, Status};

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

#[cfg(test)]
mod test;
