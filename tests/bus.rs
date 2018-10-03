// Copyright 2018 Ed McCardell
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use robo6502::{Cpu, Sys};

use self::common::*;

use self::Cycle::*;

mod common;

#[test]
fn bus() {
    let mut tested = [false; 256];

    // Ignore KIL
    let kil: [usize; 12] = [
        0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xb2, 0xd2, 0xf2,
    ];
    for op in kil.iter() {
        tested[*op] = true;
    }

    // Regular ops
    for op in 0..256 {
        if let Some(tests) = op_bus(op as u8) {
            for test in tests {
                run_test(test_cpu(), test, &mut tested);
            }
        }
    }

    // Branch ops
    for op in 0..256 {
        if let AddrMode::REL = ADDR_MODES[op] {
            test_rel(op as u8, &mut tested);
        }
    }

    test_jmp(&mut tested); // JMP $nnnn and JMP ($nnnn)
    test_push(0x08, 0x32, &mut tested); // PHP
    test_push(0x48, 0x40, &mut tested); // PHA
    test_pull(0x28, &mut tested); // PLP
    test_pull(0x68, &mut tested); // PLA
    test_brk(&mut tested); // BRK
    test_rti(&mut tested); // RTI
    test_jsr(&mut tested); // JSR
    test_rts(&mut tested); // RTS

    assert!(tested.iter().all(|&p| p));
}

fn run_test(cpu: Cpu, test: AddrTest, pass: &mut [bool]) {
    fn run_sys<T>(sys: &mut T, mut cpu: Cpu, test: AddrTest, pass: &mut [bool])
    where
        T: TestSys,
    {
        let desc = format!("{:02x}-{}", test.op(), test.desc);

        cpu.set_pc(test.start_pc());
        sys.run_instruction(&mut cpu);
        if cpu.pc() != test.end_pc {
            panic!(
                "end pc {:04x} instead of {:04x} for {}",
                cpu.pc(),
                test.end_pc,
                desc,
            );
        }
        pass[test.op() as usize] = true;
    }

    let desc = format!("{:02x}-{}", test.op(), test.desc);
    let sys = BusSys::new(desc, test.bus.clone());

    let mut sys1 = sys.clone();
    run_sys(&mut sys1, cpu.clone(), test.clone(), pass);
    sys1.check();

    let mut sys2 = StepFullSys::new(sys.clone());
    run_sys(&mut sys2, cpu.clone(), test.clone(), pass);
    sys2.sys.check();

    let mut sys3 = StepSys::new(sys.clone());
    run_sys(&mut sys3, cpu.clone(), test.clone(), pass);
    sys3.sys.check();
}

fn op_bus(op: u8) -> Option<Vec<AddrTest>> {
    use self::common::AddrMode::*;
    use self::common::MemAction::*;

    let (bases, action): (Vec<AddrTest>, Option<MemAction>) =
        match ADDR_MODES[op as usize] {
            IMP => (bus_imp(op), None),
            IMM => (bus_imm(op), None),
            ZP(action) => (bus_zp(op), Some(action)),
            ABS(action) => (bus_abs(op), Some(action)),
            ZPI(action) => (bus_zpi(op), Some(action)),
            ABI(action) => (bus_abi(op, action), Some(action)),
            IZX(action) => (bus_izx(op), Some(action)),
            IZY(action) => (bus_izy(op, action), Some(action)),
            _ => return None,
        };

    let tests = bases
        .iter()
        .map(|base| {
            let final_cycles = match action {
                None => vec![],
                Some(Load) => vec![R(base.addr, 0)],
                Some(Store) => vec![W(base.addr, 0x40)],
                Some(RmwInc) => {
                    vec![R(base.addr, 1), W(base.addr, 1), W(base.addr, 2)]
                }
                Some(RmwDec) => {
                    vec![R(base.addr, 2), W(base.addr, 2), W(base.addr, 1)]
                }
            };
            AddrTest {
                desc: base.desc.clone(),
                addr: base.addr,
                end_pc: base.end_pc,
                bus: [base.bus.clone(), final_cycles].concat(),
            }
        }).collect();
    Some(tests)
}

fn bus_imp(op: u8) -> Vec<AddrTest> {
    vec![AddrTest {
        desc: "implicit".to_owned(),
        bus: vec![
            R(0x0200, op),   // opcode fetch
            R(0x0201, 0x00), // dummy read
        ],
        end_pc: 0x0201,
        addr: 0x0000, // not used
    }]
}

fn bus_imm(op: u8) -> Vec<AddrTest> {
    vec![AddrTest {
        desc: "immediate".to_owned(),
        bus: vec![
            R(0x0200, op),   // opcode fetch
            R(0x0201, 0x00), // operand fetch
        ],
        end_pc: 0x0202,
        addr: 0x0000, // not used
    }]
}

fn bus_zp(op: u8) -> Vec<AddrTest> {
    vec![AddrTest {
        desc: "zp".to_owned(),
        bus: vec![
            R(0x0200, op),   // opcode fetch
            R(0x0201, 0x55), // address fetch
        ],
        end_pc: 0x0202,
        addr: 0x0055,
    }]
}

fn bus_abs(op: u8) -> Vec<AddrTest> {
    vec![AddrTest {
        desc: "abs".to_owned(),
        bus: vec![
            R(0x0200, op),   // opcode fetch
            R(0x0201, 0x30), // address lo fetch
            R(0x0202, 0x40), // address hi fetch
        ],
        end_pc: 0x0203,
        addr: 0x4030,
    }]
}

fn bus_zpi(op: u8) -> Vec<AddrTest> {
    vec![AddrTest {
        desc: "zpi".to_owned(),
        bus: vec![
            R(0x0200, op),   // opcode fetch
            R(0x0201, 0xf0), // base fetch
            R(0x00f0, 00),   // dummy read at base
        ],
        end_pc: 0x0202,
        addr: 0x30,
    }]
}

fn bus_abi(op: u8, action: MemAction) -> Vec<AddrTest> {
    let w = match action {
        MemAction::Load => false,
        _ => true,
    };

    let h = [0x9b, 0x9c, 0x9e, 0x9f].contains(&op);

    if !h {
        // normal abi
        if w {
            vec![AddrTest {
                desc: "abi-store".to_owned(),
                bus: vec![
                    R(0x0200, op),   // opcode fetch
                    R(0x0201, 0xf0), // base lo fetch
                    R(0x0202, 0x40), // base hi fetch
                    R(0x4030, 0x00), // dummy read (no carry)
                ],
                end_pc: 0x0203,
                addr: 0x4130,
            }]
        } else {
            vec![
                AddrTest {
                    desc: "abi-load".to_owned(),
                    bus: vec![
                        R(0x0200, op),   // opcode fetch
                        R(0x0201, 0x40), // base lo fetch
                        R(0x0202, 0x40), // base hi fetch
                    ],
                    end_pc: 0x0203,
                    addr: 0x4080,
                },
                AddrTest {
                    desc: "abi-load-page-cross".to_owned(),
                    bus: vec![
                        R(0x0200, op),   // opcode fetch
                        R(0x0201, 0xf0), // base lo fetch
                        R(0x0202, 0x40), // base hi fetch
                        R(0x4030, 0x00), // dummy read (no carry)
                    ],
                    end_pc: 0x0203,
                    addr: 0x4130,
                },
            ]
        }
    } else {
        // ops 0x9b, 0x9c, 0x9e, 0x9f
        vec![
            AddrTest {
                desc: "abh-store".to_owned(),
                bus: vec![
                    R(0x0200, op),   // opcode fetch
                    R(0x0201, 0x00), // base lo fetch
                    R(0x0202, 0x3f), // base hi fetch
                    R(0x3f40, 0x00), // dummy read (no carry)
                ],
                end_pc: 0x0203,
                addr: 0x3f40,
            },
            AddrTest {
                desc: "abh-store-page-cross".to_owned(),
                bus: vec![
                    R(0x0200, op),   // opcode fetch
                    R(0x0201, 0xc0), // base lo fetch
                    R(0x0202, 0xfe), // base hi fetch
                    R(0xfe00, 0x00), // dummy read (no carry)
                ],
                end_pc: 0x0203,
                addr: 0x04000,
            },
        ]
    }
}

fn bus_izx(op: u8) -> Vec<AddrTest> {
    vec![
        AddrTest {
            desc: "izx-index-wrap".to_owned(),
            bus: vec![
                R(0x0200, op),   // opcode fetch
                R(0x0201, 0xf0), // base fetch
                R(0x00f0, 0x00), // dummy read
                R(0x0030, 0x30), // vector lo fetch
                R(0x0031, 0x40), // vector hi fetch
            ],
            end_pc: 0x0202,
            addr: 0x4030,
        },
        AddrTest {
            desc: "izx-vector-wrap".to_owned(),
            bus: vec![
                R(0x0200, op),   // opcode fetch
                R(0x0201, 0xbf), // base fetch
                R(0x00bf, 0x00), // dummy read
                R(0x00ff, 0x30), // vector lo fetch
                R(0x0000, 0x40), // vector hi fetch
            ],
            end_pc: 0x0202,
            addr: 0x4030,
        },
    ]
}

fn bus_izy(op: u8, action: MemAction) -> Vec<AddrTest> {
    let w = match action {
        MemAction::Load => false,
        _ => true,
    };

    let h = op == 0x93;

    if !h {
        if w {
            vec![
                AddrTest {
                    desc: "izy-store".to_owned(),
                    bus: vec![
                        R(0x0200, op),   // opcode fetch
                        R(0x0201, 0x00), // base fetch
                        R(0x0000, 0x40), // vector lo fetch
                        R(0x0001, 0x40), // vector hi fetch
                        R(0x4080, 0x00), // dummy read
                    ],
                    end_pc: 0x0202,
                    addr: 0x4080,
                },
                AddrTest {
                    desc: "izy-store-page-cross".to_owned(),
                    bus: vec![
                        R(0x0200, op),   // opcode fetch
                        R(0x0201, 0xff), // base fetch
                        R(0x00ff, 0xf0), // vector lo fetch
                        R(0x0000, 0x40), // vector hi fetch
                        R(0x4030, 0x00), // dummy read (no carry)
                    ],
                    end_pc: 0x0202,
                    addr: 0x4130,
                },
            ]
        } else {
            vec![
                AddrTest {
                    desc: "izy-load".to_owned(),
                    bus: vec![
                        R(0x0200, op),   // opcode fetch
                        R(0x0201, 0x00), // base fetch
                        R(0x0000, 0x40), // vector lo fetch
                        R(0x0001, 0x40), // vector hi fetch
                    ],
                    end_pc: 0x0202,
                    addr: 0x4080,
                },
                AddrTest {
                    desc: "izy-load-page-cross".to_owned(),
                    bus: vec![
                        R(0x0200, op),   // opcode fetch
                        R(0x0201, 0xff), // base fetch
                        R(0x00ff, 0xf0), // vector lo fetch
                        R(0x0000, 0x40), // vector hi fetch
                        R(0x4030, 0x00), // dummy read (no carry)
                    ],
                    end_pc: 0x0202,
                    addr: 0x4130,
                },
            ]
        }
    } else {
        vec![AddrTest {
            desc: "izh-store".to_owned(),
            bus: vec![
                R(0x0200, op),   // opcode fetch
                R(0x0201, 0x00), // base fetch
                R(0x0000, 0x40), // vector lo fetch
                R(0x0001, 0x3f), // vector hi fetch
                R(0x3f80, 0x00), // dummy read
            ],
            end_pc: 0x0202,
            addr: 0x3f80,
        }]
    }
}

fn test_rel(op: u8, pass: &mut [bool]) {
    let (flag, set) = branch_flag(op);
    let mut skip = test_cpu();
    let mut take = test_cpu();
    skip.set_flag(flag, !set);
    take.set_flag(flag, set);

    run_test(
        skip,
        AddrTest {
            desc: "branch-skip".to_owned(),
            bus: vec![
                R(0x0200, op),   // opcode fetch
                R(0x0201, 0x10), // offset fetch
            ],
            end_pc: 0x0202,
            addr: 0x0000,
        },
        pass,
    );

    run_test(
        take.clone(),
        AddrTest {
            desc: "branch-forward".to_owned(),
            bus: vec![
                R(0x0200, op),   // opcode fetch
                R(0x0201, 0x10), // offset fetch
                R(0x0202, 0x00), // dummy read
            ],
            end_pc: 0x0212,
            addr: 0x0000,
        },
        pass,
    );

    run_test(
        take.clone(),
        AddrTest {
            desc: "branch-backward".to_owned(),
            bus: vec![
                R(0x0210, op),   // opcode fetch
                R(0x0211, 0xf0), // offset fetch
                R(0x0212, 0x00), // dummy read
            ],
            end_pc: 0x0202,
            addr: 0x0000,
        },
        pass,
    );

    run_test(
        take.clone(),
        AddrTest {
            desc: "branch-forward-page-cross".to_owned(),
            bus: vec![
                R(0x02f0, op),   // opcode fetch
                R(0x02f1, 0x10), // offset fetch
                R(0x02f2, 0x00), // dummy read
                R(0x0202, 0x00), // dummy read no carry
            ],
            end_pc: 0x0302,
            addr: 0x0000,
        },
        pass,
    );

    run_test(
        take.clone(),
        AddrTest {
            desc: "branch-backward-page-cross".to_owned(),
            bus: vec![
                R(0x0300, op),   // opcode fetch
                R(0x0301, 0xfd), // offset fetch
                R(0x0302, 0x00), // dummy read
                R(0x03ff, 0x00), // dummy read no carry
            ],
            end_pc: 0x02ff,
            addr: 0x0000,
        },
        pass,
    );
}

fn test_jmp(pass: &mut [bool]) {
    run_test(
        test_cpu(),
        AddrTest {
            desc: "jmp".to_owned(),
            bus: vec![
                R(0x0200, 0x4c), // opcode fetch
                R(0x0201, 0x55), // vector lo fetch
                R(0x0202, 0xaa), // vector hi fetch
            ],
            end_pc: 0xaa55,
            addr: 0x0000,
        },
        pass,
    );

    run_test(
        test_cpu(),
        AddrTest {
            desc: "jmp-indirect".to_owned(),
            bus: vec![
                R(0x0200, 0x6c), // opcode fetch
                R(0x0201, 0x22), // base lo fetch
                R(0x0202, 0x44), // base hi fetch
                R(0x4422, 0x55), // vector lo fetch
                R(0x4423, 0xaa), // vector hi fetch
            ],
            end_pc: 0xaa55,
            addr: 0x0000,
        },
        pass,
    );

    run_test(
        test_cpu(),
        AddrTest {
            desc: "jmp-indirect-ff-bug".to_owned(),
            bus: vec![
                R(0x0200, 0x6c), // opcode fetch
                R(0x0201, 0xff), // base lo fetch
                R(0x0202, 0x44), // base hi fetch
                R(0x44ff, 0x55), // vector lo fetch
                R(0x4400, 0xaa), // vector hi fetch
            ],
            end_pc: 0xaa55,
            addr: 0x0000,
        },
        pass,
    );
}

fn test_push(op: u8, val: u8, pass: &mut [bool]) {
    run_test(
        test_cpu(),
        AddrTest {
            desc: "push".to_owned(),
            bus: vec![
                R(0x0200, op),   // opcode fetch
                R(0x0201, 0x00), // dummy read
                W(0x0100, val),  // stack push
            ],
            end_pc: 0x0201,
            addr: 0x0000,
        },
        pass,
    );
}

fn test_pull(op: u8, pass: &mut [bool]) {
    run_test(
        test_cpu(),
        AddrTest {
            desc: "pull".to_owned(),
            bus: vec![
                R(0x0200, op),   // opcode fetch
                R(0x0201, 0x00), // dummy read
                R(0x0100, 0x00), // dummy stack read
                R(0x0101, 0x32), // stack pop
            ],
            end_pc: 0x0201,
            addr: 0x0000,
        },
        pass,
    );
}

fn test_brk(pass: &mut [bool]) {
    run_test(
        test_cpu(),
        AddrTest {
            desc: "".to_owned(),
            bus: vec![
                R(0x0200, 0x00), // opcode fetch
                R(0x0201, 0x00), // dummy operand fetch
                W(0x0100, 0x02), // write pc hi to stack
                W(0x01ff, 0x02), // write pc lo to stack
                W(0x01fe, 0x32), // write status to stack
                R(0xfffe, 0x55), // vector lo fetch
                R(0xffff, 0xaa), // vector hi fetch
            ],
            end_pc: 0xaa55,
            addr: 0x0000,
        },
        pass,
    );
}

fn test_rti(pass: &mut [bool]) {
    run_test(
        test_cpu(),
        AddrTest {
            desc: "".to_owned(),
            bus: vec![
                R(0x0200, 0x40), // opcode fetch
                R(0x0201, 0x00), // dummy read
                R(0x0100, 0x00), // dummy stack read
                R(0x0101, 0x32), // status from stack
                R(0x0102, 0x55), // pc lo from stack
                R(0x0103, 0xaa), // pc hi from stack
            ],
            end_pc: 0xaa55,
            addr: 0x0000,
        },
        pass,
    );
}

fn test_jsr(pass: &mut [bool]) {
    run_test(
        test_cpu(),
        AddrTest {
            desc: "".to_owned(),
            bus: vec![
                R(0x0200, 0x20), // opcode fetch
                R(0x0201, 0x55), // vector lo fetch
                R(0x0100, 0x00), // dummy stack read
                W(0x0100, 0x02), // pc hi to stack
                W(0x01ff, 0x02), // pc lo to stack
                R(0x0202, 0xaa), // vector hi fetch
            ],
            end_pc: 0xaa55,
            addr: 0x0000,
        },
        pass,
    );
}

fn test_rts(pass: &mut [bool]) {
    run_test(
        test_cpu(),
        AddrTest {
            desc: "".to_owned(),
            bus: vec![
                R(0x0200, 0x60), // opcode fetch
                R(0x0201, 0x00), // dummy read
                R(0x0100, 0x00), // dummy stack read
                R(0x0101, 0x55), // pc lo from stack
                R(0x0102, 0xaa), // pc hi from stack
                R(0xaa55, 0x00), // dummy read
            ],
            end_pc: 0xaa56,
            addr: 0x0000,
        },
        pass,
    );
}

fn test_cpu() -> Cpu {
    let mut cpu = Cpu::standard();
    cpu.set_a(0x40);
    cpu.set_x(0x40);
    cpu.set_y(0x40);
    cpu
}

#[derive(Clone)]
struct AddrTest {
    desc: String,
    bus: Vec<Cycle>,
    end_pc: u16,
    addr: u16,
}

impl AddrTest {
    fn op(&self) -> u8 {
        match self.bus[0] {
            R(_, op) => op,
            _ => unreachable!(),
        }
    }

    fn start_pc(&self) -> u16 {
        match self.bus[0] {
            R(pc, _) => pc,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
enum Cycle {
    R(u16, u8),
    W(u16, u8),
}

#[derive(Clone)]
struct BusSys {
    desc: String,
    bus: Vec<Cycle>,
    cycles: usize,
}

impl BusSys {
    fn new(desc: String, bus: Vec<Cycle>) -> BusSys {
        let cycles = 0;
        BusSys { cycles, desc, bus }
    }

    fn check(&self) {
        if self.cycles < self.bus.len() {
            panic!(
                "{} instead of {} cycles for {}",
                self.cycles,
                self.bus.len(),
                self.desc
            );
        }
    }
}

impl TestSys for BusSys {
    fn run_instruction(&mut self, cpu: &mut Cpu) {
        cpu.run_instruction(self);
    }
}

impl Sys for BusSys {
    fn read(&mut self, addr: u16) -> Option<u8> {
        use self::Cycle::*;
        if self.cycles >= self.bus.len() {
            panic!(
                "Extra cycle ({}, expected {}) for {}",
                self.cycles + 1,
                self.bus.len(),
                self.desc
            );
        }
        let v = match self.bus[self.cycles] {
            W(_, _) => panic!(
                "Read on write cycle ({}) for {}",
                self.cycles + 1,
                self.desc
            ),
            R(a, _) if a != addr => panic!(
                "Read at {:04x}, expected {:04x} on cycle {} for {}",
                addr,
                a,
                self.cycles + 1,
                self.desc
            ),
            R(_, v) => v,
        };
        self.cycles += 1;
        Some(v)
    }

    fn write(&mut self, addr: u16, val: u8) -> Option<()> {
        use self::Cycle::*;
        if self.cycles >= self.bus.len() {
            panic!(
                "Extra cycle ({}, expected {}) for {}",
                self.cycles + 1,
                self.bus.len(),
                self.desc
            );
        }
        match self.bus[self.cycles] {
            R(_, _) => panic!(
                "Write on read cycle ({}) for {}",
                self.cycles, self.desc
            ),
            W(a, _) if a != addr => panic!(
                "Write at {:04x}, expected {:04x} on cycle {} for {}",
                addr,
                a,
                self.cycles + 1,
                self.desc
            ),
            W(_, v) if v != val => panic!(
                "Write val {:02x}, expected {:02x} on cycle {} for {}",
                val,
                v,
                self.cycles + 1,
                self.desc
            ),
            _ => (),
        }
        self.cycles += 1;
        Some(())
    }
}
