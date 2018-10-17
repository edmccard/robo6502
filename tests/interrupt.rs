// Copyright 2018 Ed McCardell
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use robo6502::{Cmos, Cpu, NmiLength, Nmos, Status, Sys};

use self::common::*;

mod common;

#[test]
fn hijack() {
    hijack_impl(test_cpu_nmos());
    hijack_impl(test_cpu_cmos());
}

fn hijack_impl<C: Cpu>(cpu: C) {
    // needed for run_test
    let mut tested = [false; 256];

    let mut test = no_delay(
        ("irq-brk".to_owned(), vec![0x00], 0),
        -1,
        false,
        NmiLength::Plenty,
    );
    test.exp_pc[1..].copy_from_slice(&[0x0400, 0x0401, 0x0402, 0x0403]);
    run_test(cpu.clone(), 5, test, &mut tested);

    for cyc in 1..6 {
        let test = IntTest {
            desc: format!("nmi-irq-{}", cyc),
            exp_pc: vec![0x0200, 0x0201, 0x0300, 0x0301, 0x0302],
            mem: make_mem(vec![0xea]),
            nmi_on: Some(cyc),
            irq_on: Some(-1),
            nmi_length: NmiLength::Plenty,
        };
        run_test(cpu.clone(), 5, test, &mut tested);
    }
    for cyc in 6..9 {
        let test = IntTest {
            desc: format!("nmi-irq-{}", cyc),
            exp_pc: vec![0x0200, 0x0201, 0x0400, 0x0401, 0x0300],
            mem: make_mem(vec![0xea]),
            nmi_on: Some(cyc),
            irq_on: Some(-1),
            nmi_length: NmiLength::Plenty,
        };
        run_test(cpu.clone(), 5, test, &mut tested);
    }

    if cpu.is_nmos() {
        // NMI hijacks BRK in NMOS
        for cyc in 0..4 {
            let test = IntTest {
                desc: format!("nmi-brk-{}", cyc),
                exp_pc: vec![0x0200, 0x0300, 0x0301, 0x0302, 0x0303],
                mem: make_mem(vec![0x00]),
                nmi_on: Some(cyc),
                irq_on: None,
                nmi_length: NmiLength::Plenty,
            };
            run_test(cpu.clone(), 5, test, &mut tested);
        }
    } else {
        // NMI does not hijack BRK in CMOS
        for cyc in 0..4 {
            let test = IntTest {
                desc: format!("nmi-brk-{}", cyc),
                exp_pc: vec![0x0200, 0x0400, 0x0401, 0x0300, 0x0301],
                mem: make_mem(vec![0x00]),
                nmi_on: Some(cyc),
                irq_on: None,
                nmi_length: NmiLength::Plenty,
            };
            run_test(cpu.clone(), 5, test, &mut tested);
        }
    }
    for cyc in 5..8 {
        let test = IntTest {
            desc: format!("nmi-brk-{}", cyc),
            exp_pc: vec![0x0200, 0x0400, 0x0401, 0x0300, 0x0301],
            mem: make_mem(vec![0x00]),
            nmi_on: Some(cyc),
            irq_on: None,
            nmi_length: NmiLength::Plenty,
        };
        run_test(cpu.clone(), 5, test, &mut tested);
    }
}

#[test]
fn flag_change() {
    flag_change_impl(test_cpu_nmos());
    flag_change_impl(test_cpu_cmos());
}

fn flag_change_impl<C: Cpu>(cpu: C) {
    // needed for run_test
    let mut tested = [false; 256];

    let mut id_on = cpu.clone();
    let id_off = cpu.clone();
    id_on.set_flag(Status::I, true);

    // CLI and PLP (with I flag off on stack) turn off I flag
    // after polling
    let test = delay(
        ("cli".to_owned(), vec![0x58], 0),
        -1,
        false,
        NmiLength::Plenty,
    );
    run_test(id_on.clone(), 5, test, &mut tested);

    let test = delay(
        ("plp".to_owned(), vec![0x28], 0),
        -1,
        false,
        NmiLength::Plenty,
    );
    run_test(id_on.clone(), 5, test, &mut tested);

    // RTI turns off I flag before polling
    let test = no_delay(
        ("rti".to_owned(), vec![0x40], 0),
        -1,
        false,
        NmiLength::Plenty,
    );
    run_test(id_on.clone(), 5, test, &mut tested);

    // SEI turns on I flag after polling (so the irq is serviced, and
    // without delay)
    let test = no_delay(
        ("sei".to_owned(), vec![0x78], 0),
        -1,
        false,
        NmiLength::Plenty,
    );
    run_test(id_off.clone(), 5, test, &mut tested);

    // PLP (with I flag set on stack) turns on I before polling
    // (so there is no delay)
    let mut test = no_delay(
        ("sei".to_owned(), vec![0x78], 0),
        -1,
        false,
        NmiLength::Plenty,
    );
    test.mem[0x0101] = 0x34;
    run_test(id_off.clone(), 5, test, &mut tested);
}

#[test]
fn polling() {
    polling_impl(test_cpu_nmos());
    polling_impl(test_cpu_cmos());
}

fn polling_impl<C: Cpu>(cpu: C) {
    let mut tested = [false; 256];

    // BRK is tested elsewhere
    tested[0x00] = true;

    if cpu.is_nmos() {
        // Skip KIL
        let kil: [usize; 12] = [
            0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xb2, 0xd2,
            0xf2,
        ];
        for op in kil.iter() {
            tested[*op] = true;
        }
    }

    // Regular ops
    for op in 0..256 {
        if let Some(tests) = op_intr(op as u8, cpu.is_nmos()) {
            for test in tests {
                run_test(cpu.clone(), 5, test, &mut tested);
            }
        }
    }

    if !cpu.is_nmos() {
        let ops: [u8; 18] = [
            0x61, 0x65, 0x69, 0x6d, 0x71, 0x72, 0x75, 0x79, 0x7d, 0xe1, 0xe5,
            0xe9, 0xed, 0xf1, 0xf2, 0xf5, 0xf9, 0xfd,
        ];
        for op in ops.iter() {
            test_decimal_mode(cpu.clone(), *op, &mut tested);
        }
    }

    // Branch ops
    for op in 0..256 {
        use self::common::CpuAddrMode::*;
        let addr_mode = match ADDR_MODES[op] {
            S(mode) => mode,
            D(nmode, cmode) => {
                if cpu.is_nmos() {
                    nmode
                } else {
                    cmode
                }
            }
        };
        if let AddrMode::REL = addr_mode {
            test_rel(cpu.clone(), op as u8, &mut tested);
        }
    }

    test_push(cpu.clone(), 0x08, &mut tested); // PHP
    test_push(cpu.clone(), 0x48, &mut tested); // PHA
    if !cpu.is_nmos() {
        test_push(cpu.clone(), 0xda, &mut tested); // PHX
        test_push(cpu.clone(), 0x5a, &mut tested); // PHY
    }
    test_pull(cpu.clone(), 0x28, &mut tested); // PLP
    test_pull(cpu.clone(), 0x68, &mut tested); // PLA
    if !cpu.is_nmos() {
        test_pull(cpu.clone(), 0xfa, &mut tested); // PLX
        test_pull(cpu.clone(), 0x7a, &mut tested); // PLY
    }
    test_jmp(cpu.clone(), &mut tested); // JMP $nnnn and JMP ($nnnn)
    test_rti(cpu.clone(), &mut tested); // RTI
    test_jsr(cpu.clone(), &mut tested); // JSR
    test_rts(cpu.clone(), &mut tested); // RTS

    for (i, t) in tested.iter().enumerate() {
        if !t {
            println!("MISSED: {:02x}", i);
        }
    }
    assert!(tested.iter().all(|&p| p));
}

#[test]
fn swallowed_nmi() {
    let cpu = test_cpu_nmos();

    // needed for run_test
    let mut tested = [false; 256];

    for cyc in 4..6 {
        let test = IntTest {
            desc: format!("nmi-swallow-{}", cyc),
            exp_pc: vec![0x0200, 0x0400, 0x0401, 0x0402, 0x0403],
            mem: make_mem(vec![0x00]),
            nmi_on: Some(cyc),
            irq_on: None,
            nmi_length: match cyc {
                4 => NmiLength::Two,
                5 => NmiLength::One,
                _ => unreachable!(),
            },
        };
        run_test(cpu.clone(), 5, test, &mut tested);
    }

    for cyc in 6..8 {
        let test = IntTest {
            desc: format!("nmi-swallow-{}", cyc),
            exp_pc: vec![0x0200, 0x0201, 0x0400, 0x0401, 0x0402],
            mem: make_mem(vec![0xea]),
            nmi_on: Some(cyc),
            irq_on: Some(-1),
            nmi_length: match cyc {
                6 => NmiLength::Two,
                7 => NmiLength::One,
                _ => unreachable!(),
            },
        };
        run_test(cpu.clone(), 5, test, &mut tested);
    }
}

fn test_decimal_mode<C: Cpu>(mut cpu: C, op: u8, pass: &mut [bool]) {
    use self::common::AddrMode::*;
    use self::common::CpuAddrMode::*;

    let addr_mode = match ADDR_MODES[op as usize] {
        S(mode) => mode,
        D(_, mode) => mode,
    };
    let bases: Vec<CycleTest> = match addr_mode {
        DECIMM => cycles_imm(op),
        ZP(_) => cycles_zp(op),
        ABS(_) => cycles_abs(op),
        ZPI(_) => cycles_zpi(op),
        IZX(_) => cycles_izx(op),
        ABI(_) => cycles_abi(op, MemAction::Load),
        IZY(_) => cycles_izy(op, MemAction::Load),
        IZP(_) => cycles_izp(op),
        _ => unreachable!(),
    };

    let tests: Vec<CycleTest> = bases
        .iter()
        .map(|base| (base.0.clone(), base.1.clone(), base.2 + 1))
        .collect();

    cpu.set_flag(Status::D, true);
    for test in tests {
        for test in delay_tests(test) {
            run_test(cpu.clone(), 5, test, pass);
        }
    }
}

fn op_intr(op: u8, is_nmos: bool) -> Option<Vec<IntTest>> {
    use self::common::AddrMode::*;
    use self::common::CpuAddrMode::*;
    use self::common::MemAction::*;

    let addr_mode = match ADDR_MODES[op as usize] {
        S(mode) => mode,
        D(nmode, cmode) => {
            if is_nmos {
                nmode
            } else {
                cmode
            }
        }
    };
    let (bases, action): (Vec<CycleTest>, Option<MemAction>) = match addr_mode {
        IMP => (cycles_imp(op), None),
        IMM => (cycles_imm(op), None),
        DECIMM => (cycles_imm(op), None),
        ZP(action) => (cycles_zp(op), Some(action)),
        ABS(action) => (cycles_abs(op), Some(action)),
        ZPI(action) => (cycles_zpi(op), Some(action)),
        IZX(action) => (cycles_izx(op), Some(action)),
        ABI(action) => (cycles_abi(op, action), Some(action)),
        IZY(action) => (cycles_izy(op, action), Some(action)),
        IZP(action) => (cycles_izp(op), Some(action)),
        ABX(action) => (cycles_abi(op, Load), Some(action)),
        NONE => (cycles_nop1(op), None),
        _ => match op {
            0x5c => (cycles_nop8(op), None),
            0xdc | 0xfc => (cycles_abs(op), None),
            _ => return None,
        },
    };

    let tests: Vec<CycleTest> = bases
        .iter()
        .map(|base| {
            let final_cycles = match action {
                Some(RmwInc) => 2,
                Some(RmwDec) => 2,
                Some(RmwTsb) => 2,
                Some(RmwTrb) => 2,
                _ => 0,
            };
            (base.0.clone(), base.1.clone(), base.2 + final_cycles)
        })
        .collect();

    let mut int_tests: Vec<IntTest> = vec![];

    for test in tests {
        int_tests.append(&mut delay_tests(test));
    }
    Some(int_tests)
}

fn delay_tests(test: CycleTest) -> Vec<IntTest> {
    let op = test.1[0];
    let mut int_tests: Vec<IntTest> = vec![];
    for nmi in &[true, false] {
        if !*nmi && op == 0x78 {
            continue;
        }
        for cyc in -1..(test.2 as isize) {
            int_tests.push(no_delay(
                test.clone(),
                cyc,
                *nmi,
                NmiLength::Plenty,
            ));
        }
        int_tests.push(delay(
            test.clone(),
            test.2 as isize,
            *nmi,
            NmiLength::Plenty,
        ));
    }
    int_tests
}

fn test_rel<C: Cpu>(cpu: C, op: u8, pass: &mut [bool]) {
    let mut skip = cpu.clone();
    let mut take = cpu.clone();

    if op != 0x80 {
        let (flag, set) = branch_flag(op);
        skip.set_flag(flag, !set);
        take.set_flag(flag, set);

        let mut int_tests: Vec<IntTest> = vec![];
        let test = ("branch-skip".to_owned(), vec![op, 00], 1);
        for nmi in &[true, false] {
            for cyc in -1..1 {
                int_tests.push(no_delay(
                    test.clone(),
                    cyc,
                    *nmi,
                    NmiLength::Plenty,
                ));
            }
            int_tests.push(delay(test.clone(), 1, *nmi, NmiLength::Plenty));
        }
        for test in int_tests {
            run_test(skip.clone(), 5, test, pass);
        }
    }

    let mut int_tests: Vec<IntTest> = vec![];
    let test = ("branch-take".to_owned(), vec![op, 0x00], 1);
    for nmi in &[true, false] {
        let delay_cyc = if cpu.is_nmos() { 1 } else { 2 };
        for cyc in -1..delay_cyc {
            int_tests.push(no_delay(
                test.clone(),
                cyc,
                *nmi,
                NmiLength::Plenty,
            ));
        }
        for cyc in delay_cyc..3 {
            int_tests.push(delay(test.clone(), cyc, *nmi, NmiLength::Plenty));
        }
    }
    for test in int_tests {
        run_test(take.clone(), 5, test, pass);
    }

    let mut int_tests: Vec<IntTest> = vec![];
    let test = ("branch-take-page-cross".to_owned(), vec![op, 0xf0], 3);
    for nmi in &[true, false] {
        for cyc in -1..3 {
            let mut itest =
                no_delay(test.clone(), cyc, *nmi, NmiLength::Plenty);
            itest.exp_pc[1] = 0x01f2;
        }
        let mut itest = delay(test.clone(), 3, *nmi, NmiLength::Plenty);
        itest.exp_pc[1] = 0x01f2;
        itest.exp_pc[2] = 0x01f3;
        int_tests.push(itest);
    }
    for test in int_tests {
        run_test(take.clone(), 5, test, pass);
    }
}

fn test_pull<C: Cpu>(cpu: C, op: u8, pass: &mut [bool]) {
    let test = ("pull".to_owned(), vec![op], 3);
    for test in delay_tests(test) {
        run_test(cpu.clone(), 5, test, pass);
    }
}

fn test_push<C: Cpu>(cpu: C, op: u8, pass: &mut [bool]) {
    let test = ("push".to_owned(), vec![op], 2);
    for test in delay_tests(test) {
        run_test(cpu.clone(), 5, test, pass);
    }
}

fn test_jmp<C: Cpu>(cpu: C, pass: &mut [bool]) {
    let test = ("jmp".to_owned(), vec![0x4c, 0x03, 0x02], 2);
    for test in delay_tests(test) {
        run_test(cpu.clone(), 5, test, pass);
    }

    let test = ("jmp".to_owned(), vec![0x6c, 0x04, 0x00], 4);
    for test in delay_tests(test) {
        run_test(cpu.clone(), 5, test, pass);
    }

    if !cpu.is_nmos() {
        let test = ("jmp".to_owned(), vec![0x7c, 0x04, 0x00], 5);
        for test in delay_tests(test) {
            run_test(cpu.clone(), 5, test, pass);
        }
    }
}

fn test_rti<C: Cpu>(cpu: C, pass: &mut [bool]) {
    let test = ("rti".to_owned(), vec![0x40], 5);
    for test in delay_tests(test) {
        run_test(cpu.clone(), 5, test, pass);
    }
}

fn test_jsr<C: Cpu>(cpu: C, pass: &mut [bool]) {
    let test = ("jsr".to_owned(), vec![0x20, 0x03, 0x02], 5);
    for test in delay_tests(test) {
        run_test(cpu.clone(), 5, test, pass);
    }
}

fn test_rts<C: Cpu>(cpu: C, pass: &mut [bool]) {
    let test = ("rts".to_owned(), vec![0x60], 5);
    for test in delay_tests(test) {
        run_test(cpu.clone(), 5, test, pass);
    }
}

fn no_delay(
    test: CycleTest,
    cyc: isize,
    nmi: bool,
    nmi_length: NmiLength,
) -> IntTest {
    let op = test.1[0];
    let mut exp_pc = vec![0x0200, 0x0200 + (test.1).len() as u16];
    if nmi {
        exp_pc.extend_from_slice(&[0x0300, 0x0301, 0x0302]);
    } else {
        exp_pc.extend_from_slice(&[0x0400, 0x0401, 0x0402]);
    }
    let (desc, nmi_on, irq_on) = if nmi {
        ("nmi".to_owned(), Some(cyc), Some(cyc))
    } else {
        ("irq".to_owned(), None, Some(cyc))
    };
    let desc = format!("{:02x}-{}-{}-cyc={}", op, test.0.clone(), desc, cyc);
    IntTest {
        desc,
        exp_pc,
        mem: make_mem(test.1.clone()),
        nmi_on,
        irq_on,
        nmi_length,
    }
}

fn delay(
    test: CycleTest,
    cyc: isize,
    nmi: bool,
    nmi_length: NmiLength,
) -> IntTest {
    let op = test.1[0];
    let exp_pc = 0x0200 + (test.1).len() as u16;
    let mut exp_pc = vec![0x0200, exp_pc, exp_pc + 1];
    if nmi {
        exp_pc.extend_from_slice(&[0x0300, 0x0301]);
    } else {
        exp_pc.extend_from_slice(&[0x0400, 0x0401]);
    }
    let (desc, nmi_on, irq_on) = if nmi {
        ("nmi-delayed".to_owned(), Some(cyc), None)
    } else {
        ("irq-delayed".to_owned(), None, Some(cyc))
    };
    let desc = format!("{:02x}-{}-{}-cyc={}", op, test.0.clone(), desc, cyc);
    IntTest {
        desc,
        exp_pc,
        mem: make_mem(test.1.clone()),
        nmi_on,
        irq_on,
        nmi_length,
    }
}

fn run_test<C: Cpu>(
    mut cpu: C,
    count: usize,
    test: IntTest,
    pass: &mut [bool],
) {
    let name = if cpu.is_nmos() { "nmos" } else { "cmos" };
    let test = IntTest {
        desc: format!("{}-{}", name, test.desc),
        ..test
    };
    cpu.set_pc(test.start_pc());
    let op = test.op();
    let sys = IntSys::new(test);

    let sys1 = sys.clone();
    sys1.run(cpu.clone(), count);

    let sys2 = StepFullSys::new(sys.clone());
    sys2.run(cpu.clone(), count);

    let sys3 = StepSys::new(sys.clone());
    sys3.run(cpu.clone(), count);

    pass[op as usize] = true;
}

struct IntTest {
    desc: String,
    exp_pc: Vec<u16>,
    mem: Vec<u8>,
    nmi_on: Option<isize>,
    irq_on: Option<isize>,
    nmi_length: NmiLength,
}

impl IntTest {
    fn op(&self) -> u8 {
        self.mem[self.start_pc() as usize]
    }

    fn start_pc(&self) -> u16 {
        self.exp_pc[0]
    }
}

type CycleTest = (String, Vec<u8>, usize);

fn cycles_nop1(op: u8) -> Vec<CycleTest> {
    vec![("nop1".to_owned(), vec![op], 0)]
}

fn cycles_nop8(op: u8) -> Vec<CycleTest> {
    vec![("nop8".to_owned(), vec![op, 0x00, 0x00], 7)]
}

fn cycles_imp(op: u8) -> Vec<CycleTest> {
    vec![("imp".to_owned(), vec![op], 1)]
}

fn cycles_imm(op: u8) -> Vec<CycleTest> {
    vec![("imm".to_owned(), vec![op, 0x00], 1)]
}

fn cycles_zp(op: u8) -> Vec<CycleTest> {
    vec![("zp".to_owned(), vec![op, 0x00], 2)]
}

fn cycles_abs(op: u8) -> Vec<CycleTest> {
    vec![("abs".to_owned(), vec![op, 0x00, 0x00], 3)]
}

fn cycles_zpi(op: u8) -> Vec<CycleTest> {
    vec![("zpi".to_owned(), vec![op, 0x00], 3)]
}

fn cycles_izx(op: u8) -> Vec<CycleTest> {
    vec![("izx".to_owned(), vec![op, 0x00], 5)]
}

fn cycles_izp(op: u8) -> Vec<CycleTest> {
    vec![("izp".to_owned(), vec![op, 0x00], 4)]
}

fn cycles_abi(op: u8, action: MemAction) -> Vec<CycleTest> {
    match action {
        MemAction::Load | MemAction::Decimal => vec![
            ("abi-load".to_owned(), vec![op, 0x00, 0x00], 3),
            ("abi-load-page-cross".to_owned(), vec![op, 0xf0, 0x00], 4),
        ],
        _ => vec![("abi-store".to_owned(), vec![op, 0x00, 0x00], 4)],
    }
}

fn cycles_izy(op: u8, action: MemAction) -> Vec<CycleTest> {
    match action {
        MemAction::Load | MemAction::Decimal => vec![
            ("izy-load".to_owned(), vec![op, 0x00], 4),
            ("izy-load-page-cross".to_owned(), vec![op, 0x02], 5),
        ],
        _ => vec![("izy-store".to_owned(), vec![op, 0x00], 5)],
    }
}

fn test_cpu_nmos() -> impl Cpu {
    let mut cpu = Nmos::standard();
    cpu.set_flag(Status::I, false);
    cpu.set_x(0x40);
    cpu.set_y(0x40);
    cpu
}

fn test_cpu_cmos() -> impl Cpu {
    let mut cpu = Cmos::new();
    cpu.set_flag(Status::I, false);
    cpu.set_x(0x40);
    cpu.set_y(0x40);
    cpu
}

fn make_mem(code: Vec<u8>) -> Vec<u8> {
    let mut mem: Vec<u8> = vec![0xea; 0x10000];
    mem[0x0200..(0x0200 + code.len())].copy_from_slice(&code);
    mem[0xfffa] = 0x00;
    mem[0xfffb] = 0x03; // NMI vector 0x0300
    mem[0xfffe] = 0x00;
    mem[0xffff] = 0x04; // IRQ vector 0x0400
    mem[0x0000] = 0x00; // lo byte for no-page-cross izy
    mem[0x0002] = 0xf0; // lo byte for page-cross izy
    mem[0x0004] = 0x03; // lo byte for JMP ($nnnn)
    mem[0x0005] = 0x02; // hi byte for JMP ($nnnn)
    mem[0x0044] = 0x03; // lo byte for JMP ($nnnn,X)
    mem[0x0045] = 0x02; // hi byte for JMP ($nnnn,X)
    match code[0] {
        // setup for PLP
        0x28 => mem[0x0101] = 0x30, // status
        // setup for RTI
        0x40 => {
            mem[0x0101] = 0x30; // status
            mem[0x0102] = 0x01; // lo byte pc
            mem[0x0103] = 0x02; // hi byte pc
        }
        // setup for RTS
        0x60 => {
            mem[0x0101] = 0x00; // lo byte pc
            mem[0x0102] = 0x02; // hi byte pc
        }
        _ => (),
    }
    mem
}

#[derive(Clone)]
struct IntSys {
    desc: String,
    exp_pc: Vec<u16>,
    nmi_cycle: Option<isize>,
    irq_cycle: Option<isize>,
    mem: Vec<u8>,
    nmi_length: NmiLength,
    sync: bool,
    irq: bool,
    nmi: bool,
    cycle: usize,
    op_count: usize,
}

impl IntSys {
    fn new(test: IntTest) -> IntSys {
        #[cfg_attr(rustfmt, rustfmt_skip)]
        let mut sys = IntSys {
            desc: test.desc, exp_pc: test.exp_pc,
            nmi_cycle: test.nmi_on, irq_cycle: test.irq_on,
            nmi_length: test.nmi_length,
            sync: false, irq: false, nmi: false,
            cycle: 0, op_count: 0,
            mem: test.mem,
        };
        if let Some(cycle) = sys.nmi_cycle {
            if cycle < 0 {
                sys.nmi = true;
            }
        }
        if let Some(cycle) = sys.irq_cycle {
            if cycle < 0 {
                sys.irq = true;
            }
        }
        sys
    }

    fn tick(&mut self) {
        if let Some(cycle) = self.nmi_cycle {
            if cycle as usize == self.cycle {
                self.nmi = true;
            }
        }
        if let Some(cycle) = self.irq_cycle {
            if cycle as usize == self.cycle {
                self.irq = true;
            }
        }
        self.cycle += 1;
    }
}

impl Sys for IntSys {
    fn set_sync(&mut self, set: bool) {
        self.sync = set;
    }

    fn irq(&self) -> bool {
        self.irq
    }

    fn peek_nmi(&self) -> bool {
        self.nmi
    }

    fn poll_nmi(&mut self) -> bool {
        let poll = self.nmi;
        self.nmi = false;
        poll
    }

    fn nmi_length(&self) -> NmiLength {
        self.nmi_length
    }

    fn read(&mut self, addr: u16) -> Option<u8> {
        self.tick();
        if self.sync {
            let exp_pc = self.exp_pc[self.op_count];
            if addr != exp_pc {
                panic!(
                    "{}: expected {:04x}, got {:04x} on cycle {}",
                    self.desc,
                    exp_pc,
                    addr,
                    self.cycle - 1
                );
            }
            self.op_count += 1;
        }
        Some(self.mem[addr as usize])
    }

    fn write(&mut self, addr: u16, val: u8) -> Option<()> {
        self.tick();
        self.mem[addr as usize] = val;
        Some(())
    }
}

impl TestSys for IntSys {
    fn run_instruction<C: Cpu>(&mut self, cpu: &mut C) {
        cpu.run_instruction(self);
    }
}

trait IntRun {
    fn run<C: Cpu>(self, cpu: C, count: usize);
}

impl<T: TestSys> IntRun for T {
    fn run<C: Cpu>(mut self, mut cpu: C, count: usize) {
        for _ in 0..count {
            self.run_instruction(&mut cpu);
        }
    }
}
