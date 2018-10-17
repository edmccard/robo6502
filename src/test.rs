// Copyright 2018 Ed McCardell
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use super::*;

#[test]
fn reset() {
    let mut cpu = Nmos::standard();
    cpu.reset();
    let mut sys = NullSys::new(0x2000, 0x4000, 0x6000);
    cpu.run_instruction(&mut sys);
    assert_eq!(cpu.pc(), 0x4000);
    assert!(cpu.flag(Status::I));
}

struct NullSys {
    nmi_vec: u16,
    res_vec: u16,
    irq_vec: u16,
}

impl NullSys {
    fn new(nmi_vec: u16, res_vec: u16, irq_vec: u16) -> NullSys {
        NullSys {
            nmi_vec,
            res_vec,
            irq_vec,
        }
    }
}

impl Sys for NullSys {
    fn read(&mut self, addr: u16) -> Option<u8> {
        let val = match addr {
            0xfffa => self.nmi_vec as u8,
            0xfffb => (self.nmi_vec >> 8) as u8,
            0xfffc => self.res_vec as u8,
            0xfffd => (self.res_vec >> 8) as u8,
            0xfffe => self.irq_vec as u8,
            0xffff => (self.irq_vec >> 8) as u8,
            _ => 0xff,
        };
        Some(val)
    }

    fn write(&mut self, _: u16, _: u8) -> Option<()> {
        Some(())
    }
}
