#![feature(duration_as_u128)]

use std::io::Read;
use std::time::{Duration, Instant};

use robo6502::{Cpu, Sys};

fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("6502_functional_test.bin");
    let mut file = std::fs::File::open(path).unwrap();

    let mut mem = Vec::new();
    file.read_to_end(&mut mem).unwrap();
    let mut sys = VecSys { mem };

    let mut cpu = Cpu::standard();
    cpu.set_pc(0x0400);

    let now = Instant::now();
    loop {
        cpu.run_instruction(&mut sys);
        if cpu.pc() == 0x3469 {
            break;
        }
    }
    let now = now.elapsed();
    println!("MHz: {}", 96241364.0 / (total_time(now) * 1000000.0));
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
