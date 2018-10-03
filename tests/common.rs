#![allow(dead_code)]

use machine_int::MachineInt;

use robo6502::{Cpu, NmiLength, Status, Sys};

pub trait TestSys: Sys {
    fn run_instruction(&mut self, cpu: &mut Cpu);
}

pub trait MemSys: Sys {
    fn mem(&self) -> &[u8];
}

pub struct VecSys {
    mem: Vec<u8>,
}

impl VecSys {
    pub fn new(mem: Vec<u8>) -> VecSys {
        assert!(mem.len() >= 0xffff);
        VecSys { mem }
    }
}

impl MemSys for VecSys {
    #[inline]
    fn mem(&self) -> &[u8] {
        &self.mem
    }
}

impl TestSys for VecSys {
    #[inline]
    fn run_instruction(&mut self, cpu: &mut Cpu) {
        cpu.run_instruction(self);
    }
}

impl Sys for VecSys {
    #[inline]
    fn read(&mut self, addr: u16) -> Option<u8> {
        Some(self.mem[addr as usize])
    }

    #[inline]
    fn write(&mut self, addr: u16, val: u8) -> Option<()> {
        self.mem[addr as usize] = val;
        Some(())
    }
}

pub struct CycleSys<T: Sys> {
    pub sys: T,
    pub cycles: MachineInt<u64>,
}

impl<T: Sys> CycleSys<T> {
    pub fn new(sys: T) -> CycleSys<T> {
        CycleSys {
            sys,
            cycles: MachineInt(0),
        }
    }
}

impl<T: Sys> Sys for CycleSys<T> {
    fn read(&mut self, addr: u16) -> Option<u8> {
        let result = self.sys.read(addr);
        self.cycles += 1;
        result
    }

    fn write(&mut self, addr: u16, val: u8) -> Option<()> {
        let result = self.sys.write(addr, val);
        self.cycles += 1;
        result
    }
}

pub struct StepFullSys<T: Sys> {
    pub sys: T,
    do_stop: bool,
    sync: bool,
}

impl<T: Sys> StepFullSys<T> {
    pub fn new(sys: T) -> StepFullSys<T> {
        StepFullSys {
            sys,
            do_stop: false,
            sync: false,
        }
    }
}

impl<T: MemSys> MemSys for StepFullSys<T> {
    fn mem(&self) -> &[u8] {
        self.sys.mem()
    }
}

impl<T: Sys> TestSys for StepFullSys<T> {
    #[inline]
    fn run_instruction(&mut self, cpu: &mut Cpu) {
        cpu.run_instruction(self);
        cpu.run_instruction(self);
    }
}

impl<T: Sys> Sys for StepFullSys<T> {
    #[inline]
    fn read(&mut self, addr: u16) -> Option<u8> {
        if self.sync {
            self.do_stop = true;
        } else {
            if self.do_stop {
                self.do_stop = false;
                return None;
            }
        }
        self.sys.read(addr)
    }

    #[inline]
    fn write(&mut self, addr: u16, val: u8) -> Option<()> {
        // No instruction writes immediately after opcode fetch, so we
        // don't have to check do_stop here.
        self.sys.write(addr, val)
    }

    #[inline]
    fn set_sync(&mut self, set: bool) {
        self.sync = set;
        self.sys.set_sync(set);
    }

    #[inline]
    fn poll_nmi(&mut self) -> bool {
        self.sys.poll_nmi()
    }

    #[inline]
    fn peek_nmi(&self) -> bool {
        self.sys.peek_nmi()
    }

    #[inline]
    fn irq(&self) -> bool {
        self.sys.irq()
    }

    #[inline]
    fn nmi_length(&self) -> NmiLength {
        self.sys.nmi_length()
    }
}

pub struct StepSys<T: Sys> {
    pub sys: T,
    do_stop: bool,
    sync: bool,
}

impl<T: Sys> StepSys<T> {
    pub fn new(sys: T) -> StepSys<T> {
        StepSys {
            sys,
            do_stop: false,
            sync: false,
        }
    }
}

impl<T: MemSys> MemSys for StepSys<T> {
    fn mem(&self) -> &[u8] {
        self.sys.mem()
    }
}

impl<T: Sys> TestSys for StepSys<T> {
    #[inline]
    fn run_instruction(&mut self, cpu: &mut Cpu) {
        cpu.run_instruction(self);
        loop {
            if self.sync {
                break;
            }
            cpu.run_instruction(self);
        }
    }
}

impl<T: Sys> Sys for StepSys<T> {
    #[inline]
    fn read(&mut self, addr: u16) -> Option<u8> {
        self.do_stop = !self.do_stop;
        if !self.do_stop {
            return None;
        }
        self.sys.read(addr)
    }

    #[inline]
    fn write(&mut self, addr: u16, val: u8) -> Option<()> {
        self.do_stop = !self.do_stop;
        if !self.do_stop {
            return None;
        }
        self.sys.write(addr, val)
    }

    #[inline]
    fn set_sync(&mut self, set: bool) {
        self.sync = set;
        self.sys.set_sync(set);
    }

    #[inline]
    fn irq(&self) -> bool {
        self.sys.irq()
    }

    #[inline]
    fn poll_nmi(&mut self) -> bool {
        self.sys.poll_nmi()
    }

    #[inline]
    fn peek_nmi(&self) -> bool {
        self.sys.peek_nmi()
    }

    #[inline]
    fn nmi_length(&self) -> NmiLength {
        self.sys.nmi_length()
    }
}

#[derive(Clone, Copy)]
pub enum MemAction {
    Load,
    Store,
    RmwInc,
    RmwDec,
}

#[derive(Clone, Copy)]
pub enum AddrMode {
    IMP,
    IMM,
    ZP(MemAction),
    ABS(MemAction),
    ZPI(MemAction),
    ABI(MemAction),
    IZX(MemAction),
    IZY(MemAction),
    REL,
    MISC,
}

use self::AddrMode::*;
use self::MemAction::*;
#[cfg_attr(rustfmt, rustfmt_skip)]
pub static ADDR_MODES: [AddrMode; 256] = [
    MISC,        IZX(Load),   MISC,        IZX(RmwInc),
    ZP(Load),    ZP(Load),    ZP(RmwInc),  ZP(RmwInc),
    MISC,        IMM,         IMP,         IMM,
    ABS(Load),   ABS(Load),   ABS(RmwInc), ABS(RmwInc),

    REL,         IZY(Load),   MISC,        IZY(RmwInc),
    ZPI(Load),   ZPI(Load),   ZPI(RmwInc), ZPI(RmwInc),
    IMP,         ABI(Load),   IMP,         ABI(RmwInc),
    ABI(Load),   ABI(Load),   ABI(RmwInc), ABI(RmwInc),

    MISC,        IZX(Load),   MISC,        IZX(RmwInc),
    ZP(Load),    ZP(Load),    ZP(RmwInc),  ZP(RmwInc),
    MISC,        IMM,         IMP,         IMM,
    ABS(Load),   ABS(Load),   ABS(RmwInc), ABS(RmwInc),

    REL,         IZY(Load),   MISC,        IZY(RmwInc),
    ZPI(Load),   ZPI(Load),   ZPI(RmwInc), ZPI(RmwInc),
    IMP,         ABI(Load),   IMP,         ABI(RmwInc),
    ABI(Load),   ABI(Load),   ABI(RmwInc), ABI(RmwInc),

    MISC,        IZX(Load),   MISC,        IZX(RmwDec),
    ZP(Load),    ZP(Load),    ZP(RmwDec),  ZP(RmwDec),
    MISC,        IMM,         IMP,         IMM,
    MISC,        ABS(Load),   ABS(RmwDec), ABS(RmwDec),

    REL,         IZY(Load),   MISC,        IZY(RmwDec),
    ZPI(Load),   ZPI(Load),   ZPI(RmwDec), ZPI(RmwDec),
    IMP,         ABI(Load),   IMP,         ABI(RmwDec),
    ABI(Load),   ABI(Load),   ABI(RmwDec), ABI(RmwDec),

    MISC,        IZX(Load),   MISC,        IZX(RmwDec),
    ZP(Load),    ZP(Load),    ZP(RmwDec),  ZP(RmwDec),
    MISC,        IMM,         IMP,         IMM,
    MISC,        ABS(Load),   ABS(RmwDec), ABS(RmwDec),

    REL,         IZY(Load),   MISC,        IZY(RmwDec),
    ZPI(Load),   ZPI(Load),   ZPI(RmwDec), ZPI(RmwDec),
    IMP,         ABI(Load),   IMP,         ABI(RmwDec),
    ABI(Load),   ABI(Load),   ABI(RmwDec), ABI(RmwDec),

    IMM,         IZX(Store),  IMM,         IZX(Store),
    ZP(Store),   ZP(Store),   ZP(Store),   ZP(Store),
    IMP,         IMM,         IMP,         IMM,
    ABS(Store),  ABS(Store),  ABS(Store),  ABS(Store),

    REL,         IZY(Store),  MISC,        IZY(Store),
    ZPI(Store),  ZPI(Store),  ZPI(Store),  ZPI(Store),
    IMP,         ABI(Store),  IMP,         ABI(Store),
    ABI(Store),  ABI(Store),  ABI(Store),  ABI(Store),

    IMM,         IZX(Load),   IMM,         IZX(Load),
    ZP(Load),    ZP(Load),    ZP(Load),    ZP(Load),
    IMP,         IMM,         IMP,         IMM,
    ABS(Load),   ABS(Load),   ABS(Load),   ABS(Load),

    REL,         IZY(Load),   MISC,        IZY(Load),
    ZPI(Load),   ZPI(Load),   ZPI(Load),   ZPI(Load),
    IMP,         ABI(Load),   IMP,         ABI(Load),
    ABI(Load),   ABI(Load),   ABI(Load),   ABI(Load),

    IMM,         IZX(Load),   IMM,         IZX(RmwDec),
    ZP(Load),    ZP(Load),    ZP(RmwDec),  ZP(RmwDec),
    IMP,         IMM,         IMP,         IMM,
    ABS(Load),   ABS(Load),   ABS(RmwDec), ABS(RmwDec),

    REL,         IZY(Load),   MISC,        IZY(RmwDec),
    ZPI(Load),   ZPI(Load),   ZPI(RmwDec), ZPI(RmwDec),
    IMP,         ABI(Load),   IMP,         ABI(RmwDec),
    ABI(Load),   ABI(Load),   ABI(RmwDec), ABI(RmwDec),

    IMM,         IZX(Load),   IMM,         IZX(RmwInc),
    ZP(Load),    ZP(Load),    ZP(RmwInc),  ZP(RmwInc),
    IMP,         IMM,         IMP,         IMM,
    ABS(Load),   ABS(Load),   ABS(RmwInc), ABS(RmwInc),

    REL,         IZY(Load),   MISC,        IZY(RmwInc),
    ZPI(Load),   ZPI(Load),   ZPI(RmwInc), ZPI(RmwInc),

    IMP,         ABI(Load),   IMP,         ABI(RmwInc),
    ABI(Load),   ABI(Load),   ABI(RmwInc), ABI(RmwInc),
];

pub fn branch_flag(op: u8) -> (Status, bool) {
    match op {
        0x10 => (Status::N, false), // BPL
        0x30 => (Status::N, true),  // BMI
        0x50 => (Status::V, false), // BVC
        0x70 => (Status::V, true),  // BVS
        0x90 => (Status::C, false), // BCC
        0xb0 => (Status::C, true),  // BCS
        0xd0 => (Status::Z, false), // BNE
        0xf0 => (Status::Z, true),  // BEQ
        _ => unreachable!(),
    }
}
