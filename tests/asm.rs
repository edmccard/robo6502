// Copyright 2018 Ed McCardell
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::io::Read;

use robo6502::Cpu;

use self::common::{MemSys, StepFullSys, StepSys, TestSys, VecSys};

mod common;

#[test]
#[ignore]
fn klaus2m5_functional() {
    fn run<T: TestSys>(mut sys: T) {
        let mut cpu = Cpu::standard();
        cpu.set_pc(0x0400);
        let mut prev_pc = cpu.pc();
        // 96241367
        for _i in 0..31000000 {
            sys.run_instruction(&mut cpu);
            if cpu.pc() == prev_pc {
                if cpu.pc() == 0x3469 {
                    break;
                }
                panic!("pc loop at {:04x}", cpu.pc());
            } else {
                prev_pc = cpu.pc();
            }
        }
        if cpu.pc() != 0x3469 {
            panic!("test failed");
        }
    }

    let data = load_bin("6502_functional_test.bin", 0);

    let sys = VecSys::new(data.clone());
    run(sys);

    let sys = StepFullSys::new(VecSys::new(data.clone()));
    run(sys);

    let sys = StepSys::new(VecSys::new(data.clone()));
    run(sys);
}

#[test]
#[ignore]
fn clark_decimal() {
    fn run<T: TestSys + MemSys>(mut sys: T) {
        let mut cpu = Cpu::standard();
        cpu.set_pc(0x0200);
        for _i in 0..18000000 {
            sys.run_instruction(&mut cpu);
            if cpu.pc() == 0x024b {
                break;
            }
        }
        if cpu.pc() != 0x024b {
            panic!("test failed -- end pc at {:04x}", cpu.pc());
        }
        if sys.mem()[0x000b] != 0 {
            panic!("test failed");
        }
    }

    let data = load_bin("clark_decimal_test.bin", 0x0200);

    let sys = VecSys::new(data.clone());
    run(sys);

    let sys = StepFullSys::new(VecSys::new(data.clone()));
    run(sys);

    let sys = StepSys::new(VecSys::new(data.clone()));
    run(sys);
}

#[test]
#[ignore]
fn kevtris_nestest() {
    fn run<T: TestSys + MemSys>(mut sys: T) {
        let mut cpu = Cpu::nes();
        cpu.reset();
        sys.run_instruction(&mut cpu);
        cpu.set_pc(0xc000);
        // 8970 8980
        for _i in 0..8990 {
            sys.run_instruction(&mut cpu);
        }
        if cpu.pc() != 0xc66e {
            panic!("test failed -- end pc at {:04x}", cpu.pc());
        }
        if sys.mem()[0x10] != 0 {
            panic!("test failed -- error {:02x} in 0x0010", sys.mem()[0x10]);
        }
        if sys.mem()[0x11] != 0 {
            panic!("test failed -- error {:02x} in 0x0010", sys.mem()[0x11]);
        }
    }

    let data = load_nes("nestest.nes");

    let sys = VecSys::new(data.clone());
    run(sys);

    let sys = StepFullSys::new(VecSys::new(data.clone()));
    run(sys);

    let sys = StepSys::new(VecSys::new(data.clone()));
    run(sys);
}

fn load_bin(name: &str, base: usize) -> Vec<u8> {
    let bin = test_file(name);
    let pre = vec![0u8; base];
    let post = vec![0u8; 65536 - (pre.len() + bin.len())];
    [pre, bin, post].concat()
}

fn load_nes(name: &str) -> Vec<u8> {
    let nes = test_file(name);
    let mut bin = vec![0u8; 0x10000];
    bin[0x8000..0xc000].copy_from_slice(&nes[0x0010..0x4010]);
    bin[0xc000..].copy_from_slice(&nes[0x0010..0x4010]);
    bin
}

fn test_file(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name);
    let mut file = std::fs::File::open(path).unwrap();
    let mut bin = Vec::new();
    file.read_to_end(&mut bin).unwrap();
    bin
}
