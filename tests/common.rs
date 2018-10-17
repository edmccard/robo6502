#![allow(dead_code)]

use machine_int::MachineInt;

use robo6502::{Cpu, NmiLength, Status, Sys};

pub trait TestSys: Sys {
    fn run_instruction<C: Cpu>(&mut self, cpu: &mut C);
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
    fn run_instruction<C: Cpu>(&mut self, cpu: &mut C) {
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
    fn run_instruction<C: Cpu>(&mut self, cpu: &mut C) {
        cpu.run_instruction(self);
        if cpu.partial_inst() {
            cpu.run_instruction(self);
        }
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
    fn run_instruction<C: Cpu>(&mut self, cpu: &mut C) {
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
    Stz,     // CMOS STZ
    Decimal, // CMOS ADC/SBC
    RmwInc,
    RmwDec,
    RmwTsb, // CMOS TSB
    RmwTrb, // CMOS TRB
}

#[derive(Clone, Copy)]
pub enum AddrMode {
    IMP,
    IMM,
    DECIMM, // CMOS ADC #nn/SBC #nn
    ZP(MemAction),
    ABS(MemAction),
    ZPI(MemAction),
    ABI(MemAction),
    ABX(MemAction), // CMOS ASL, LSR, ROL, ROR
    IZX(MemAction),
    IZY(MemAction),
    IZP(MemAction), // CMOS ($nn)
    REL,
    MISC,
    NONE, // CMOS single-cycle NOP
}

#[derive(Clone, Copy)]
pub enum CpuAddrMode {
    S(AddrMode),
    D(AddrMode, AddrMode),
}

use self::AddrMode::*;
use self::CpuAddrMode::*;
use self::MemAction::*;
#[cfg_attr(rustfmt, rustfmt_skip)]
pub static ADDR_MODES: [CpuAddrMode; 256] = [
    S(MISC),
    S(IZX(Load)),
    D(MISC, IMM),
    D(IZX(RmwInc), NONE),
    D(ZP(Load), ZP(RmwTsb)),
    S(ZP(Load)),
    S(ZP(RmwInc)),
    D(ZP(RmwInc), NONE),
    S(MISC),
    S(IMM),
    S(IMP),
    D(IMM, NONE),
    D(ABS(Load), ABS(RmwTsb)),
    S(ABS(Load)),
    S(ABS(RmwInc)),
    D(ABS(RmwInc), NONE),

    S(REL),
    S(IZY(Load)),
    D(MISC, IZP(Load)),
    D(IZY(RmwInc), NONE),
    D(ZPI(Load), ZP(RmwTrb)),
    S(ZPI(Load)),
    S(ZPI(RmwInc)),
    D(ZPI(RmwInc), NONE),
    S(IMP),
    S(ABI(Load)),
    S(IMP),
    D(ABI(RmwInc), NONE),
    D(ABI(Load), ABS(RmwTrb)),
    S(ABI(Load)),
    D(ABI(RmwInc), ABX(RmwInc)),
    D(ABI(RmwInc), NONE),

    S(MISC),
    S(IZX(Load)),
    D(MISC, IMM),
    D(IZX(RmwInc), NONE),
    S(ZP(Load)),
    S(ZP(Load)),
    S(ZP(RmwInc)),
    D(ZP(RmwInc), NONE),
    S(MISC),
    S(IMM),
    S(IMP),
    D(IMM, NONE),
    S(ABS(Load)),
    S(ABS(Load)),
    S(ABS(RmwInc)),
    D(ABS(RmwInc), NONE),

    S(REL),
    S(IZY(Load)),
    D(MISC, IZP(Load)),
    D(IZY(RmwInc), NONE),
    S(ZPI(Load)),
    S(ZPI(Load)),
    S(ZPI(RmwInc)),
    D(ZPI(RmwInc), NONE),
    S(IMP),
    S(ABI(Load)),
    S(IMP),
    D(ABI(RmwInc), NONE),
    S(ABI(Load)),
    S(ABI(Load)),
    D(ABI(RmwInc), ABX(RmwInc)),
    D(ABI(RmwInc), NONE),

    S(MISC),
    S(IZX(Load)),
    D(MISC, IMM),
    D(IZX(RmwDec), NONE),
    S(ZP(Load)),
    S(ZP(Load)),
    S(ZP(RmwDec)),
    D(ZP(RmwDec), NONE),
    S(MISC),
    S(IMM),
    S(IMP),
    D(IMM, NONE),
    S(MISC),
    S(ABS(Load)),
    S(ABS(RmwDec)),
    D(ABS(RmwDec), NONE),

    S(REL),
    S(IZY(Load)),
    D(MISC, IZP(Load)),
    D(IZY(RmwDec), NONE),
    S(ZPI(Load)),
    S(ZPI(Load)),
    S(ZPI(RmwDec)),
    D(ZPI(RmwDec), NONE),
    S(IMP),
    S(ABI(Load)),
    D(IMP, MISC),
    D(ABI(RmwDec), NONE),
    D(ABI(Load), MISC),
    S(ABI(Load)),
    D(ABI(RmwDec), ABX(RmwDec)),
    D(ABI(RmwDec), NONE),

    S(MISC),
    D(IZX(Load), IZX(Decimal)),
    D(MISC, IMM),
    D(IZX(RmwDec), NONE),
    D(ZP(Load), ZP(Stz)),
    D(ZP(Load), ZP(Decimal)),
    S(ZP(RmwDec)),
    D(ZP(RmwDec), NONE),
    S(MISC),
    D(IMM, DECIMM),
    S(IMP),
    D(IMM, NONE),
    S(MISC),
    D(ABS(Load), ABS(Decimal)),
    S(ABS(RmwDec)),
    D(ABS(RmwDec), NONE),

    S(REL),
    D(IZY(Load), IZY(Decimal)),
    D(MISC, IZP(Decimal)),
    D(IZY(RmwDec), NONE),
    D(ZPI(Load), ZPI(Stz)),
    D(ZPI(Load), ZPI(Decimal)),
    S(ZPI(RmwDec)),
    D(ZPI(RmwDec), NONE),
    S(IMP),
    D(ABI(Load), ABI(Decimal)),
    D(IMP, MISC),
    D(ABI(RmwDec), NONE),
    D(ABI(Load), MISC),
    D(ABI(Load), ABI(Decimal)),
    D(ABI(RmwDec), ABX(RmwDec)),
    D(ABI(RmwDec), NONE),

    D(IMM, REL),
    S(IZX(Store)),
    S(IMM),
    D(IZX(Store), NONE),
    S(ZP(Store)),
    S(ZP(Store)),
    S(ZP(Store)),
    D(ZP(Store), NONE),
    S(IMP),
    S(IMM),
    S(IMP),
    D(IMM, NONE),
    S(ABS(Store)),
    S(ABS(Store)),
    S(ABS(Store)),
    D(ABS(Store), NONE),

    S(REL),
    S(IZY(Store)),
    D(MISC, IZP(Store)),
    D(IZY(Store), NONE),
    S(ZPI(Store)),
    S(ZPI(Store)),
    S(ZPI(Store)),
    D(ZPI(Store), NONE),
    S(IMP),
    S(ABI(Store)),
    S(IMP),
    D(ABI(Store), NONE),
    D(ABI(Store), ABS(Stz)),
    S(ABI(Store)),
    D(ABI(Store), ABI(Stz)),
    D(ABI(Store), NONE),

    S(IMM),
    S(IZX(Load)),
    S(IMM),
    D(IZX(Load), NONE),
    S(ZP(Load)),
    S(ZP(Load)),
    S(ZP(Load)),
    D(ZP(Load), NONE),
    S(IMP),
    S(IMM),
    S(IMP),
    D(IMM, NONE),
    S(ABS(Load)),
    S(ABS(Load)),
    S(ABS(Load)),
    D(ABS(Load), NONE),

    S(REL),
    S(IZY(Load)),
    D(MISC, IZP(Load)),
    D(IZY(Load), NONE),
    S(ZPI(Load)),
    S(ZPI(Load)),
    S(ZPI(Load)),
    D(ZPI(Load), NONE),
    S(IMP),
    S(ABI(Load)),
    S(IMP),
    D(ABI(Load), NONE),
    S(ABI(Load)),
    S(ABI(Load)),
    S(ABI(Load)),
    D(ABI(Load), NONE),

    S(IMM),
    S(IZX(Load)),
    S(IMM),
    D(IZX(RmwDec), NONE),
    S(ZP(Load)),
    S(ZP(Load)),
    S(ZP(RmwDec)),
    D(ZP(RmwDec), NONE),
    S(IMP),
    S(IMM),
    S(IMP),
    D(IMM, NONE),
    S(ABS(Load)),
    S(ABS(Load)),
    S(ABS(RmwDec)),
    D(ABS(RmwDec), NONE),

    S(REL),
    S(IZY(Load)),
    D(MISC, IZP(Load)),
    D(IZY(RmwDec), NONE),
    S(ZPI(Load)),
    S(ZPI(Load)),
    S(ZPI(RmwDec)),
    D(ZPI(RmwDec), NONE),
    S(IMP),
    S(ABI(Load)),
    D(IMP, MISC),
    D(ABI(RmwDec), NONE),
    D(ABI(Load), MISC),
    S(ABI(Load)),
    S(ABI(RmwDec)),
    D(ABI(RmwDec), NONE),

    S(IMM),
    D(IZX(Load), IZX(Decimal)),
    S(IMM),
    D(IZX(RmwInc), NONE),
    S(ZP(Load)),
    D(ZP(Load), ZP(Decimal)),
    S(ZP(RmwInc)),
    D(ZP(RmwInc), NONE),
    S(IMP),
    D(IMM, DECIMM),
    S(IMP),
    D(IMM, NONE),
    S(ABS(Load)),
    D(ABS(Load), ABS(Decimal)),
    S(ABS(RmwInc)),
    D(ABS(RmwInc), NONE),

    S(REL),
    D(IZY(Load), IZY(Decimal)),
    D(MISC, IZP(Decimal)),
    D(IZY(RmwInc), NONE),
    S(ZPI(Load)),
    D(ZPI(Load), ZPI(Decimal)),
    S(ZPI(RmwInc)),
    D(ZPI(RmwInc), NONE),
    S(IMP),
    D(ABI(Load), ABI(Decimal)),
    D(IMP, MISC),
    D(ABI(RmwInc), NONE),
    D(ABI(Load), MISC),
    D(ABI(Load), ABI(Decimal)),
    S(ABI(RmwInc)),
    D(ABI(RmwInc), NONE),
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
