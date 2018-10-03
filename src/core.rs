// Copyright 2018 Ed McCardell
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;

use machine_int::{AsFrom, MachineInt};

use crate::Sys;

mod ops;

#[derive(Clone)]
pub struct Cpu {
    flags: Flags,
    op_step: MachineInt<u32>,
    pc: Addr,
    base1: Addr,
    op: u8,
    lo_byte: Byte,
    a: Byte,
    x: Byte,
    y: Byte,
    sp: Byte,
    latch: bool,
    nmi_edge: bool,
    reset: bool,
    halted: bool,
    has_decimal: bool,
}

impl Cpu {
    pub fn standard() -> Cpu {
        Cpu {
            flags: Default::default(),
            op_step: Default::default(),
            pc: Default::default(),
            base1: Default::default(),
            op: 0,
            lo_byte: Default::default(),
            a: Default::default(),
            x: Default::default(),
            y: Default::default(),
            sp: Default::default(),
            latch: false,
            nmi_edge: false,
            reset: false,
            halted: false,
            has_decimal: true,
        }
    }

    pub fn nes() -> Cpu {
        let mut cpu = Cpu::standard();
        cpu.has_decimal = false;
        cpu
    }

    #[inline]
    pub fn run_instruction<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 0 {
            sys.set_sync(true);
            if self.latch {
                self.read(sys, self.pc)?;
                self.op = 0x00;
            } else {
                self.op = self.fetch_operand(sys)?.0;
            }
            sys.set_sync(false);
            self.exec(sys)?;
        } else {
            self.step_exec(sys)?;
        }
        self.op_step = MachineInt(0);
        Some(())
    }

    pub fn reset(&mut self) {
        self.reset = true;
        self.latch = true;
        self.halted = false;
    }

    #[inline]
    pub fn pc(&self) -> u16 {
        self.pc.0
    }

    #[inline]
    pub fn set_pc(&mut self, val: u16) {
        self.pc.0 = val;
    }

    #[inline]
    pub fn sp(&self) -> u8 {
        self.sp.0
    }

    #[inline]
    pub fn set_sp(&mut self, val: u8) {
        self.sp.0 = val;
    }

    #[inline]
    pub fn a(&self) -> u8 {
        self.a.0
    }

    #[inline]
    pub fn set_a(&mut self, val: u8) {
        self.a.0 = val;
    }

    #[inline]
    pub fn x(&self) -> u8 {
        self.x.0
    }

    #[inline]
    pub fn set_x(&mut self, val: u8) {
        self.x.0 = val;
    }

    #[inline]
    pub fn y(&self) -> u8 {
        self.y.0
    }

    #[inline]
    pub fn set_y(&mut self, val: u8) {
        self.y.0 = val;
    }

    #[inline]
    pub fn halted(&self) -> bool {
        self.halted
    }

    #[inline]
    pub fn status(&self) -> u8 {
        self.flags.to_byte().0
    }

    #[inline]
    pub fn set_status(&mut self, val: u8) {
        self.flags.from_byte(val.into());
    }

    #[inline]
    pub fn flag(&self, f: Status) -> bool {
        match f {
            Status::N => self.flags.n(),
            Status::V => self.flags.v(),
            Status::D => self.flags.d(),
            Status::I => self.flags.i(),
            Status::Z => self.flags.z(),
            Status::C => self.flags.c(),
        }
    }

    #[inline]
    pub fn set_flag(&mut self, f: Status, set: bool) {
        match f {
            Status::N => self.flags.set_n(set),
            Status::V => self.flags.set_v(set),
            Status::D => self.flags.set_d(set),
            Status::I => self.flags.set_i(set),
            Status::Z => self.flags.set_z(set),
            Status::C => self.flags.set_c(set),
        };
    }
}

#[allow(non_snake_case)]
impl Cpu {
    #[inline]
    fn ADC(&mut self, val: Byte) {
        if self.flags.d && self.has_decimal {
            self.ADC_dec(Word::from(val));
        } else {
            self.ADC_hex(Word::from(val));
        }
    }

    #[inline]
    fn ADC_hex(&mut self, val: Word) {
        let sum = self.a + val + self.flags.c;
        let v = !(self.a ^ val) & (val ^ sum) & 0x80;
        self.flags.v = v.lo();
        self.flags.set_c(sum > 0xff);
        self.a = sum.lo();
        self.flags.nz(self.a);
    }

    #[inline]
    fn ADC_dec(&mut self, val: Word) {
        let mut sl = (self.a & 0x0f) + (val & 0x0f) + self.flags.c;
        if sl >= 0x0a {
            sl = ((sl + 0x06) & 0x0f) + 0x10;
        }
        let mut sum = (self.a & 0xf0) + (val & 0xf0) + sl;
        let v = !(self.a ^ val) & (val ^ sum) & 0x80;
        self.flags.v = v.lo();
        self.flags.n = sum.lo();

        self.flags.z = (self.a + val + self.flags.c).lo();

        if sum >= 0xa0 {
            sum += 0x60;
        }
        self.flags.set_c(sum >= 0x100);
        self.a = sum.lo();
    }

    #[inline]
    fn AND(&mut self, val: Byte) {
        self.a &= val;
        self.flags.nz(self.a);
    }

    #[inline]
    fn ARR(&mut self, val: Byte) {
        self.a &= val;
        let arr = (self.a >> 1) | (self.flags.c << 7);
        self.flags.nz(arr);

        if !self.flags.d {
            self.ARR_hex(arr);
        } else {
            self.ARR_dec(arr);
        }
    }

    #[inline]
    fn ARR_hex(&mut self, val: Byte) {
        self.flags.set_c((val & 0x40) != 0);
        self.flags.v = (val & 0x40) ^ ((val & 0x20) << 1);
        self.a = val;
    }

    #[inline]
    fn ARR_dec(&mut self, mut val: Byte) {
        self.flags.v = (val ^ self.a) & 0x40;
        if (self.a & 0x0f) >= 0x05 {
            val = ((val + 0x06) & 0x0f) | (val & 0xf0);
        }
        self.flags.set_c((self.a & 0xf0) >= 0x50);
        if self.flags.c() {
            val += 0x60;
        }
        self.a = val;
    }

    #[inline]
    fn ASL(&mut self, val: Byte) -> Byte {
        self.flags.c = val >> 7;
        let val = val << 1;
        self.flags.nz(val);
        val
    }

    #[inline]
    fn BIT(&mut self, val: Byte) {
        let r = self.a & val;
        self.flags.z = r;
        self.flags.v = val & 0x40;
        self.flags.n = val;
    }

    #[inline]
    fn CMP(&mut self, reg: Byte, val: Byte) {
        let r = reg - val;
        self.flags.nz(r);
        self.flags.set_c(val <= reg);
    }

    #[inline]
    fn DEC(&mut self, val: Byte) -> Byte {
        let val = val - 1;
        self.flags.nz(val);
        val
    }

    #[inline]
    fn EOR(&mut self, val: Byte) {
        self.a ^= val;
        self.flags.nz(self.a);
    }

    #[inline]
    fn INC(&mut self, val: Byte) -> Byte {
        let val = val + 1;
        self.flags.nz(val);
        val
    }

    #[inline]
    fn LSR(&mut self, val: Byte) -> Byte {
        self.flags.c = val & 1;
        let val = val >> 1;
        self.flags.nz(val);
        val
    }

    #[inline]
    fn ORA(&mut self, val: Byte) {
        self.a |= val;
        self.flags.nz(self.a);
    }

    #[inline]
    fn ROL(&mut self, val: Byte) -> Byte {
        let c = self.flags.c;
        self.flags.c = val >> 7;
        let val = (val << 1) | c;
        self.flags.nz(val);
        val
    }

    #[inline]
    fn ROR(&mut self, val: Byte) -> Byte {
        let c = self.flags.c << 7;
        self.flags.c = val & 1;
        let val = (val >> 1) | c;
        self.flags.nz(val);
        val
    }

    #[inline]
    fn SBC(&mut self, val: Byte) {
        if self.flags.d && self.has_decimal {
            self.SBC_dec(Word::from(val));
        } else {
            self.SBC_hex(Word::from(val));
        }
    }

    #[inline]
    fn SBC_hex(&mut self, val: Word) {
        let diff = self.a - val - (!self.flags.c() as u16);
        let v = (self.a ^ diff) & (self.a ^ val) & 0x80;
        self.flags.v = v.lo();
        self.flags.set_c(diff < 0x100);
        self.a = diff.lo();
        self.flags.nz(self.a);
    }

    #[inline]
    fn SBC_dec(&mut self, val: Word) {
        let bdiff = self.a - val - (!self.flags.c() as u16);
        let v = (self.a ^ bdiff) & (self.a ^ val) & 0x80;
        self.flags.v = v.lo();
        self.flags.nz(bdiff.lo());

        let val = SignedWord::as_from(val);
        let mut dl = (self.a & 0x0f) - (val & 0x0f) - (!self.flags.c() as i16);
        if dl < 0 {
            dl = ((dl - 0x06) & 0x0f) - 0x10;
        }
        let mut diff = (self.a & 0xf0) - (val & 0xf0) + dl;
        if diff < 0 {
            diff -= 0x60;
        }

        self.flags.set_c(bdiff < 0x100);
        self.a = Byte::as_from(diff);
    }
}

impl Cpu {
    fn addr_zp<S: Sys>(&mut self, sys: &mut S) -> Option<Addr> {
        Some(Addr::zp(self.fetch_operand(sys)?))
    }

    fn addr_zpi<S: Sys>(&mut self, sys: &mut S, reg: Byte) -> Option<Addr> {
        self.base1 = self.addr_zp(sys)?;
        self.read(sys, self.base1)?;
        Some(self.base1.no_carry(reg))
    }

    fn addr_abs<S: Sys>(&mut self, sys: &mut S) -> Option<Addr> {
        self.lo_byte = self.fetch_operand(sys)?;
        let hi_byte = self.fetch_operand(sys)?;
        Some(Addr::from_bytes(self.lo_byte, hi_byte))
    }

    fn addr_abi<S: Sys>(
        &mut self,
        sys: &mut S,
        reg: Byte,
        write: bool,
    ) -> Option<Addr> {
        self.base1 = self.addr_abs(sys)?;
        if write || self.base1.check_carry(reg) {
            self.read(sys, self.base1.no_carry(reg))?;
        } else {
            self.op_step += 1;
        }
        Some(self.base1 + reg)
    }

    fn addr_izx<S: Sys>(&mut self, sys: &mut S) -> Option<Addr> {
        self.base1 = self.addr_zp(sys)?;
        self.read(sys, self.base1)?;
        self.base1 = self.base1.no_carry(self.x);
        Some(self.fetch_vector_zp(sys, self.base1)?)
    }

    fn addr_izy<S: Sys>(&mut self, sys: &mut S, write: bool) -> Option<Addr> {
        self.base1 = self.addr_zp(sys)?;
        self.base1 = self.fetch_vector_zp(sys, self.base1)?;
        if write || self.base1.check_carry(self.y) {
            self.read(sys, self.base1.no_carry(self.y))?;
        } else {
            self.op_step += 1;
        }
        Some(self.base1 + self.y)
    }

    fn implicit<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_signals(sys);
        self.read(sys, self.pc)?;
        Some(())
    }

    fn immediate<S: Sys>(&mut self, sys: &mut S) -> Option<Byte> {
        self.poll_signals(sys);
        self.fetch_operand(sys)
    }

    fn rmw<F, S: Sys>(&mut self, sys: &mut S, addr: Addr, op: F) -> Option<()>
    where
        F: Fn(&mut Self, Byte) -> Byte,
    {
        self.lo_byte = self.read(sys, addr)?;
        self.write(sys, addr, self.lo_byte)?;
        self.lo_byte = op(self, self.lo_byte);
        self.store(sys, addr, self.lo_byte)?;
        Some(())
    }

    fn branch<S: Sys>(&mut self, sys: &mut S, taken: bool) -> Option<()> {
        self.poll_signals(sys);
        self.lo_byte = self.fetch_operand(sys)?;
        if taken {
            self.read(sys, self.pc)?;
            let offset = BranchOffset::as_from(self.lo_byte);
            if self.pc.check_carry(offset) {
                self.poll_signals(sys);
                self.read(sys, self.pc.no_carry(offset))?;
            }
            self.pc += offset;
        }
        Some(())
    }

    fn fetch_vector_zp<S>(&mut self, sys: &mut S, zp: Addr) -> Option<Addr>
    where
        S: Sys,
    {
        //self.base2 = Addr::zp(self.read(sys, zp)?);
        self.lo_byte = self.read(sys, zp)?;
        let hi_byte = self.read(sys, zp.no_carry(1))?;
        Some(Addr::from_bytes(self.lo_byte, hi_byte))
    }

    fn fetch_operand<S: Sys>(&mut self, sys: &mut S) -> Option<Byte> {
        let val = self.read(sys, self.pc)?;
        self.pc += 1;
        Some(val)
    }

    fn halt(&mut self) -> Option<()> {
        self.halted = true;
        None
    }

    fn read<S: Sys>(&mut self, sys: &mut S, addr: Addr) -> Option<Byte> {
        let val = sys.read(addr.0)?;
        self.op_step += 1;
        Some(MachineInt(val))
    }

    fn load<S: Sys>(&mut self, sys: &mut S, addr: Addr) -> Option<Byte> {
        self.poll_signals(sys);
        self.read(sys, addr)
    }

    fn write<S>(&mut self, sys: &mut S, addr: Addr, val: Byte) -> Option<()>
    where
        S: Sys,
    {
        sys.write(addr.0, val.0)?;
        self.op_step += 1;
        Some(())
    }

    fn store<S>(&mut self, sys: &mut S, addr: Addr, val: Byte) -> Option<()>
    where
        S: Sys,
    {
        self.poll_signals(sys);
        self.write(sys, addr, val)
    }

    fn read_stack<S: Sys>(&mut self, sys: &mut S) -> Option<Byte> {
        self.read(sys, Addr::stack(self.sp))
    }

    fn write_stack<S: Sys>(&mut self, sys: &mut S, val: Byte) -> Option<()> {
        self.write(sys, Addr::stack(self.sp), val)
    }
}

impl Cpu {
    fn step_addr_zpi<S: Sys>(
        &mut self,
        sys: &mut S,
        reg: Byte,
    ) -> Option<Addr> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        self.read(sys, self.base1)?;
        Some(self.base1.no_carry(reg))
        // op_step == 3
    }

    fn step_addr_abs<S: Sys>(&mut self, sys: &mut S) -> Option<Addr> {
        if self.op_step == 1 {
            self.lo_byte = self.fetch_operand(sys)?;
        }

        // op_step == 2
        let hi_byte = self.fetch_operand(sys)?;
        Some(Addr::from_bytes(self.lo_byte, hi_byte))
        // op_step == 3
    }

    fn step_addr_abi<S: Sys>(
        &mut self,
        sys: &mut S,
        reg: Byte,
        write: bool,
    ) -> Option<Addr> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        if write || self.base1.check_carry(reg) {
            self.read(sys, self.base1.no_carry(reg))?;
        } else {
            self.op_step += 1;
        }
        Some(self.base1 + reg)
        // op_step == 4
    }

    fn step_addr_izx<S: Sys>(&mut self, sys: &mut S) -> Option<Addr> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }
        if self.op_step == 2 {
            self.read(sys, self.base1)?;
            self.base1 = self.base1.no_carry(self.x);
        }

        // op_step >= 3
        Some(self.step_fetch_vector_zp(sys, self.base1, 3)?)
        // op_step == 5
    }

    fn step_addr_izy<S: Sys>(
        &mut self,
        sys: &mut S,
        write: bool,
    ) -> Option<Addr> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }
        if self.op_step < 4 {
            self.base1 = self.step_fetch_vector_zp(sys, self.base1, 2)?;
        }

        // op_step == 4
        if write || self.base1.check_carry(self.y) {
            self.read(sys, self.base1.no_carry(self.y))?;
        } else {
            self.op_step += 1;
        }
        Some(self.base1 + self.y)
        // op_step == 5
    }

    fn step_fetch_vector_zp<S: Sys>(
        &mut self,
        sys: &mut S,
        zp: Addr,
        start_step: u32,
    ) -> Option<Addr> {
        let start_step = MachineInt(start_step);
        // start_state is 3 from izx, 2 from izy
        if self.op_step == start_step {
            self.lo_byte = self.read(sys, zp)?;
        }
        // op_step == start_step + 1
        let hi_byte = self.read(sys, zp.no_carry(1))?;
        Some(Addr::from_bytes(self.lo_byte, hi_byte))
        // op_step == 5(izx), 4(izy)
    }

    fn step_rmw<F, S: Sys>(
        &mut self,
        sys: &mut S,
        addr: Addr,
        op: F,
        start_step: u32,
    ) -> Option<()>
    where
        F: Fn(&mut Self, Byte) -> Byte,
    {
        let start_step = MachineInt(start_step);
        if self.op_step == start_step {
            self.lo_byte = self.read(sys, addr)?;
        }
        if self.op_step == start_step + 1 {
            self.write(sys, addr, self.lo_byte)?;
            self.lo_byte = op(self, self.lo_byte);
        }
        // op_step == start_step + 2
        self.store(sys, addr, self.lo_byte)
    }

    fn step_branch<S: Sys>(&mut self, sys: &mut S, taken: bool) -> Option<()> {
        if self.op_step == 1 {
            self.poll_signals(sys);
            self.lo_byte = self.fetch_operand(sys)?;
        }

        // op_step >= 2
        if taken {
            if self.op_step == 2 {
                self.read(sys, self.pc)?;
            }
            if self.op_step == 3 {
                let offset = BranchOffset::as_from(self.lo_byte);
                if self.pc.check_carry(offset) {
                    self.poll_signals(sys);
                    self.read(sys, self.pc.no_carry(offset))?;
                }
                self.pc += offset;
            }
        }
        Some(())
    }

    fn step_halt<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.fetch_operand(sys)?;
        }

        if self.op_step == 2 {
            self.read(sys, MachineInt(0xffff))?;
        }

        if self.op_step == 3 {
            self.read(sys, MachineInt(0xfffe))?;
        }

        if self.op_step == 4 {
            self.read(sys, MachineInt(0xfffe))?;
        }

        // op_step >= 5
        self.read(sys, MachineInt(0xffff))?;
        self.op_step -= 1;
        Some(())
    }
}

impl fmt::Debug for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "PC: {:04x} A: {:02x} X: {:02x} Y: {:02x} S: {:02x} P: {:?}",
            self.pc, self.a.0, self.x.0, self.y.0, self.sp.0, self.flags,
        )
    }
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

impl Cpu {
    fn poll_signals<S: Sys>(&mut self, sys: &mut S) {
        if sys.poll_nmi() {
            self.nmi_edge = true;
        }
        let irq = (!self.flags.i) && sys.irq();
        self.latch = self.nmi_edge || irq || self.reset;
    }

    fn clear_signals(&mut self) {
        self.reset = false;
        self.nmi_edge = false;
        self.latch = false;
    }

    fn signal_vector<S: Sys>(&self, sys: &mut S) -> Addr {
        if self.reset {
            MachineInt(0xfffc)
        } else if self.nmi_edge || sys.peek_nmi() {
            MachineInt(0xfffa)
        } else {
            MachineInt(0xfffe)
        }
    }
}

#[derive(Clone, Default)]
pub struct Signals {
    nmi: bool,
    irq: bool,
    sync: bool,
}

trait AddrMath<T> {
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

trait AddrExt {
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

type Addr = MachineInt<u16>;
type SignedWord = MachineInt<i16>;
type Word = MachineInt<u16>;
type Byte = MachineInt<u8>;
type BranchOffset = MachineInt<i8>;
