use std::io::Read;
use std::iter;
use std::time::{Duration, Instant};

use robo6502::{Cmos, Cpu, Nmos, Sys};

fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("6502_functional_test.bin");
    let mut file = std::fs::File::open(path).unwrap();

    let mut mem = Vec::new();
    file.read_to_end(&mut mem).unwrap();

    let nmos = iter::repeat_with(|| run_program(mem.clone(), Nmos::standard()))
        .take(3)
        .fold(0.0 / 0.0, f64::max);
    println!("NMOS MHz: {}", nmos);
    let cmos = iter::repeat_with(|| run_program(mem.clone(), Cmos::new()))
        .take(3)
        .fold(0.0 / 0.0, f64::max);
    println!("CMOS MHz: {}", cmos);
}

fn run_program<C: Cpu>(mem: Vec<u8>, mut cpu: C) -> f64 {
    fn main_loop<C: Cpu, S: Sys>(sys: &mut S, cpu: &mut C) -> Option<()> {
        loop {
            cpu.run_instruction(sys)?;
            if cpu.pc() == 0x3469 {
                break;
            }
        }
        Some(())
    }

    let mut sys = StepSys::new(VecSys { mem });
    cpu.set_pc(0x0400);

    let now = Instant::now();
    loop {
        match main_loop(&mut sys, &mut cpu) {
            Some(()) => break,
            None => {
                if cpu.halted() {
                    panic!("Unexpected KIL instruction");
                }
            }
        }
    }
    let now = now.elapsed();

    let cycles = if cpu.is_nmos() {
        96241364.0
    } else {
        96561319.0
    };
    cycles / (total_time(now) * 1000000.0)
}

fn total_time(d: Duration) -> f64 {
    d.as_secs() as f64 + d.subsec_nanos() as f64 * 1e-9
}

struct VecSys {
    mem: Vec<u8>,
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

pub trait MemSys: Sys {
    fn mem(&self) -> &[u8];
}

impl<T: MemSys> MemSys for StepSys<T> {
    fn mem(&self) -> &[u8] {
        self.sys.mem()
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
    fn nmi_length(&self) -> robo6502::NmiLength {
        self.sys.nmi_length()
    }
}
