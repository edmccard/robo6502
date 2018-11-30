#![feature(duration_as_u128)]

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
        .fold(0.0/0.0, f64::max);
    println!("NMOS MHz: {}", nmos);
    let cmos = iter::repeat_with(|| run_program(mem.clone(), Cmos::new()))
        .take(3)
        .fold(0.0/0.0, f64::max);
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

    let mut sys = VecSys { mem };
    cpu.set_pc(0x0400);

    let now = Instant::now();
    loop {
        match main_loop(&mut sys, &mut cpu) {
            Some(()) => break,
            None => {
                if cpu.halted() {
                    panic!("Unexpected KIL instruction");
                } else {
                    panic!("Unexpected interruption");
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
