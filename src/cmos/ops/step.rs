// Copyright 2018 Ed McCardell
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use machine_int::MachineInt;

use crate::mi::{Addr, AddrExt, AddrMath};
use crate::{Cmos, Sys};

#[allow(non_snake_case)]
impl Cmos {
    // BRK
    fn step_op_00<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            if self.latch {
                self.read(sys, self.pc)?;
            } else {
                self.fetch_operand(sys)?;
            }
        }

        if self.op_step == 2 {
            if self.reset {
                self.read_stack(sys)?;
            } else {
                self.write_stack(sys, self.pc.hi())?;
            }
            self.sp -= 1;
        }

        if self.op_step == 3 {
            if self.reset {
                self.read_stack(sys)?;
            } else {
                self.write_stack(sys, self.pc.lo())?;
            }
            self.sp -= 1;
            self.base1 = self.signal_vector(sys);
        }

        if self.op_step == 4 {
            if self.reset {
                self.read_stack(sys)?;
            } else if self.latch {
                self.write_stack(sys, self.flags.to_byte() & 0b1110_1111)?;
            } else {
                self.write_stack(sys, self.flags.to_byte())?;
            }
            self.sp -= 1;
        }

        if self.op_step == 5 {
            self.lo_byte = self.read(sys, self.base1)?;
        }

        // op_step == 6
        self.hi_byte = self.read(sys, self.base1 + 1)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        self.flags.i = true;
        self.flags.d = false;
        self.clear_signals();
        Some(())
    }

    // ORA ($nn,X)
    fn step_op_01<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izx(sys)?;
        }

        // op_step == 5
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // step_op_02 = op_02
    // step_op_03 = op_03

    // TSB $nn
    fn step_op_04<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step >= 2
        self.step_rmw(sys, self.base1, Cmos::TSB, 2)
    }

    // ORA $nn
    fn step_op_05<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nn
    fn step_op_06<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step >= 2
        self.step_rmw(sys, self.base1, Cmos::ASL, 2)
    }

    // step_op_07 = op_07

    // PHP
    fn step_op_08<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.read(sys, self.pc)?;
        }

        // op_step == 2
        self.store(sys, Addr::stack(self.sp), self.flags.to_byte())?;
        self.sp -= 1;
        Some(())
    }

    // step_op_09 = op_09
    // step_op_0A = op_0A
    // step_op_0B = op_0B

    // TSB $nnnn
    fn step_op_0C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::TSB, 3)
    }

    // ORA $nnnn
    fn step_op_0D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nnnn
    fn step_op_0E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::ASL, 3)
    }

    // step_op_0F = op_0F

    // BPL
    fn step_op_10<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.step_branch(sys, !self.flags.n())
    }

    // ORA ($nn),Y
    fn step_op_11<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izy(sys, false)?;
        }

        // op_step == 5
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ORA ($nn)
    fn step_op_12<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_izp(sys)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // step_op_13 = op_13

    // TRB $nn
    fn step_op_14<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step >= 2
        self.step_rmw(sys, self.base1, Cmos::TRB, 2)
    }

    // ORA $nn,X
    fn step_op_15<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nn,X
    fn step_op_16<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::ASL, 3)
    }

    // step_op_17 = op_18
    // step_op_18 = op_18

    // ORA $nnnn,Y
    fn step_op_19<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.y, false)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // step_op_1A = op_1A
    // step_op_1B = op_1B

    // TRB $nnnn
    fn step_op_1C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::TRB, 3)
    }

    // ORA $nnnn,X
    fn step_op_1D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nnnn,X
    fn step_op_1E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step >= 4
        self.step_rmw(sys, self.base1, Cmos::ASL, 4)
    }

    // step_op_1F = op_1F

    // JSR $nnnn
    fn step_op_20<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.lo_byte = self.fetch_operand(sys)?;
        }

        if self.op_step == 2 {
            self.read_stack(sys)?;
        }

        if self.op_step == 3 {
            self.write_stack(sys, self.pc.hi())?;
            self.sp -= 1;
        }

        if self.op_step == 4 {
            self.write_stack(sys, self.pc.lo())?;
            self.sp -= 1;
            self.poll_signals(sys);
        }

        // op_step == 5
        self.hi_byte = self.fetch_operand(sys)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // AND ($nn,X)
    fn step_op_21<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izx(sys)?;
        }

        // op_step == 5
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // step_op_22 = op_22
    // step_op_23 = op_23

    // BIT $nn
    fn step_op_24<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nn
    fn step_op_25<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nn
    fn step_op_26<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step >= 2
        self.step_rmw(sys, self.base1, Cmos::ROL, 2)
    }

    // step_op_27 = op_27

    // PLP
    fn step_op_28<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.read(sys, self.pc)?;
        }

        if self.op_step == 2 {
            self.read_stack(sys)?;
            self.sp += 1;
        }

        // op_step == 3
        let p = self.load(sys, Addr::stack(self.sp))?;
        self.flags.from_byte(p);
        Some(())
    }

    // step_op_29 = op_29
    // step_op_2A = op_2A
    // step_op_2B = op_2B

    // BIT $nnnn
    fn step_op_2C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nnnn
    fn step_op_2D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nnnn
    fn step_op_2E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::ROL, 3)
    }

    // step_op_2F = op_2F

    // BMI
    fn step_op_30<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.step_branch(sys, self.flags.n())
    }

    // AND ($nn),Y
    fn step_op_31<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izy(sys, false)?;
        }

        // op_step == 5
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // AND ($nn)
    fn step_op_32<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_izp(sys)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // step_op_33 = op_33

    // BIT $nn,X
    fn step_op_34<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nn,X
    fn step_op_35<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nn,X
    fn step_op_36<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::ROL, 3)
    }

    // step_op_37 = op_37
    // step_op_38 = op_38

    // AND $nnnn,Y
    fn step_op_39<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.y, false)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // step_op_3A = op_3A
    // step_op_3B = op_3B

    // BIT $nnnn,X
    fn step_op_3C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nnnn,X
    fn step_op_3D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nnnn,X
    fn step_op_3E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step >= 4
        self.step_rmw(sys, self.base1, Cmos::ROL, 4)
    }

    // step_op_3F = op_3F

    // RTI
    fn step_op_40<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.read(sys, self.pc)?;
        }

        if self.op_step == 2 {
            self.read_stack(sys)?;
            self.sp += 1;
        }

        if self.op_step == 3 {
            let p = self.read_stack(sys)?;
            self.sp += 1;
            self.flags.from_byte(p);
        }

        if self.op_step == 4 {
            self.lo_byte = self.read_stack(sys)?;
            self.sp += 1;
            self.poll_signals(sys);
        }

        // op_step == 5
        self.hi_byte = self.read_stack(sys)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // EOR ($nn,X)
    fn step_op_41<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izx(sys)?;
        }

        // op_step == 5
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // step_op_42 = op_42
    // step_op_43 = op_43

    // NOP $nn
    fn step_op_44<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        self.load(sys, self.base1)?;
        Some(())
    }

    // EOR $nn
    fn step_op_45<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nn
    fn step_op_46<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step >= 2
        self.step_rmw(sys, self.base1, Cmos::LSR, 2)
    }

    // step_op_47 = op_47

    // PHA
    fn step_op_48<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.read(sys, self.pc)?;
        }

        // op_step == 2
        self.store(sys, Addr::stack(self.sp), self.a)?;
        self.sp -= 1;
        Some(())
    }

    // step_op_49 = op_49
    // step_op_4A = op_4A
    // step_op_4B = op_4B

    // JMP $nnnn
    fn step_op_4C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.lo_byte = self.fetch_operand(sys)?;
            self.poll_signals(sys);
        }

        // op_step == 2
        self.hi_byte = self.fetch_operand(sys)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // EOR $nnnn
    fn step_op_4D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nnnn
    fn step_op_4E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::LSR, 3)
    }

    // step_op_4F = op_4F

    // BVC
    fn step_op_50<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.step_branch(sys, !self.flags.v())
    }

    // EOR ($nn),Y
    fn step_op_51<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izy(sys, false)?;
        }

        // op_step == 5
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // EOR ($nn)
    fn step_op_52<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_izp(sys)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // step_op_53 = op_53

    // NOP $nn,X
    fn step_op_54<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        self.load(sys, self.base1)?;
        Some(())
    }

    // EOR $nn,X
    fn step_op_55<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nn,X
    fn step_op_56<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::LSR, 3)
    }

    // step_op_57 = op_57
    // step_op_58 = op_58

    // EOR $nnnn,Y
    fn step_op_59<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.y, false)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // PHY
    fn step_op_5A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.read(sys, self.pc)?;
        }

        // op_step == 2
        self.store(sys, Addr::stack(self.sp), self.y)?;
        self.sp -= 1;
        Some(())
    }

    // step_op_5B = op_5B

    // NOP (eight-cycle)
    fn step_op_5C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = Addr::from_bytes(
                self.step_addr_abs(sys)?.lo(),
                MachineInt(0xff),
            );
        }

        if self.op_step == 3 {
            self.load(sys, self.base1)?;
        }

        while self.op_step < 7 {
            self.read(sys, MachineInt(0xffff))?;
        }
        self.load(sys, MachineInt(0xffff))?;
        Some(())
    }

    // EOR $nnnn,X
    fn step_op_5D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nnnn,X
    fn step_op_5E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step >= 4
        self.step_rmw(sys, self.base1, Cmos::LSR, 4)
    }

    // step_op_5F = op_5F

    // RTS
    fn step_op_60<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.read(sys, self.pc)?;
        }

        if self.op_step == 2 {
            self.read_stack(sys)?;
            self.sp += 1;
        }

        if self.op_step == 3 {
            self.lo_byte = self.read_stack(sys)?;
            self.sp += 1;
        }

        if self.op_step == 4 {
            self.hi_byte = self.read_stack(sys)?;
            self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
            self.poll_signals(sys);
        }

        // op_step == 5
        self.fetch_operand(sys)?;
        Some(())
    }

    // ADC ($nn,X)
    fn step_op_61<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izx(sys)?;
        }

        // op_step >= 5
        self.step_decimal(sys, self.base1, Cmos::ADC, 5)
    }

    // step_op_62 = op_62
    // step_op_63 = op_63

    // NOP* $nn
    fn step_op_64<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        self.store(sys, self.base1, MachineInt(0))?;
        Some(())
    }

    // ADC $nn
    fn step_op_65<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step >= 2
        self.step_decimal(sys, self.base1, Cmos::ADC, 2)
    }

    // ROR $nn
    fn step_op_66<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step >= 2
        self.step_rmw(sys, self.base1, Cmos::ROR, 2)
    }

    // step_op_67 = op_67

    // PLA
    fn step_op_68<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.read(sys, self.pc)?;
        }

        if self.op_step == 2 {
            self.read_stack(sys)?;
            self.sp += 1;
        }

        // op_step == 3
        self.a = self.load(sys, Addr::stack(self.sp))?;
        self.flags.nz(self.a);
        Some(())
    }

    // ADC #nn
    fn step_op_69<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            if !self.flags.d {
                self.poll_signals(sys);
            }
            self.lo_byte = self.fetch_operand(sys)?;
        }

        // op_step == 2
        if self.flags.d {
            self.load(sys, self.pc)?;
        }
        self.ADC(self.lo_byte);
        Some(())
    }

    // step_op_6A = op_6A
    // step_op_6B = op_6B

    // JMP ($nnnn)
    fn step_op_6C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        if self.op_step == 3 {
            self.lo_byte = self.read(sys, self.base1)?;
        }

        // op_step == 4
        self.hi_byte = self.load(sys, self.base1 + 1)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // ADC $nnnn
    fn step_op_6D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step >= 3
        self.step_decimal(sys, self.base1, Cmos::ADC, 3)
    }

    // ROR $nnnn
    fn step_op_6E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::ROR, 3)
    }

    // step_op_6F = op_6F

    // BVS
    fn step_op_70<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.step_branch(sys, self.flags.v())
    }

    // ADC ($nn),Y
    fn step_op_71<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izy(sys, false)?;
        }

        // op_step >= 5
        self.step_decimal(sys, self.base1, Cmos::ADC, 5)
    }

    // ADC ($nn)
    fn step_op_72<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_izp(sys)?;
        }

        // op_step == 4
        self.step_decimal(sys, self.base1, Cmos::ADC, 4)
    }

    // step_op_73 = op_73

    // STZ $nn,X
    fn step_op_74<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        self.store(sys, self.base1, MachineInt(0))?;
        Some(())
    }

    // ADC $nn,X
    fn step_op_75<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step >= 3
        self.step_decimal(sys, self.base1, Cmos::ADC, 3)
    }

    // ROR $nn,X
    fn step_op_76<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::ROR, 3)
    }

    // step_op_77 = op_77
    // step_op_78 = op_78

    // ADC $nnnn,Y
    fn step_op_79<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.y, false)?;
        }

        // op_step >= 4
        self.step_decimal(sys, self.base1, Cmos::ADC, 4)
    }

    // PLY
    fn step_op_7A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.read(sys, self.pc)?;
        }

        if self.op_step == 2 {
            self.read_stack(sys)?;
            self.sp += 1;
        }

        // op_step == 3
        self.y = self.load(sys, Addr::stack(self.sp))?;
        self.flags.nz(self.y);
        Some(())
    }

    // step_op_7B = op_7B

    // JMP ($nnnn,X)
    fn step_op_7C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.lo_byte = self.fetch_operand(sys)?;
        }

        if self.op_step == 2 {
            self.hi_byte = self.read(sys, self.pc)?;
            self.base1 = self.addr() + self.x;
        }

        if self.op_step == 3 {
            self.fetch_operand(sys)?;
        }

        if self.op_step == 4 {
            self.lo_byte = self.read(sys, self.base1)?;
        }

        // op_step == 5
        self.hi_byte = self.load(sys, self.base1 + 1)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // ADC $nnnn,X
    fn step_op_7D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step == 4
        self.step_decimal(sys, self.base1, Cmos::ADC, 4)
    }

    // ROR $nnnn,X
    fn step_op_7E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step >= 4
        self.step_rmw(sys, self.base1, Cmos::ROR, 4)
    }

    // step_op_7F = op_7F

    fn step_op_80<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.step_branch(sys, true)
    }

    // STA ($nn,X)
    fn step_op_81<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izx(sys)?;
        }

        // op_step == 5
        self.store(sys, self.base1, self.a)
    }

    // step_op_82 = op_82
    // step_op_83 = op_83

    // STY $nn
    fn step_op_84<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        self.store(sys, self.base1, self.y)
    }

    // STA $nn
    fn step_op_85<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        self.store(sys, self.base1, self.a)
    }

    // STX $nn
    fn step_op_86<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        self.store(sys, self.base1, self.x)
    }

    // step_op_87 = op_87
    // step_op_88 = op_88
    // step_op_89 = op_89
    // step_op_8A = op_8A
    // step_op_8B = op_8B

    // STY $nnnn
    fn step_op_8C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        self.store(sys, self.base1, self.y)
    }

    // STA $nnnn
    fn step_op_8D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        self.store(sys, self.base1, self.a)
    }

    // STX $nnnn
    fn step_op_8E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        self.store(sys, self.base1, self.x)
    }

    // step_op_8F = op_8F

    // BCC
    fn step_op_90<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.step_branch(sys, !self.flags.c())
    }

    // STA ($nn),Y
    fn step_op_91<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izy(sys, true)?;
        }

        // op_step == 5
        self.store(sys, self.base1, self.a)
    }

    // STA ($nn)
    fn step_op_92<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_izp(sys)?;
        }

        // op_step == 4
        self.store(sys, self.base1, self.a)
    }

    // step_op_93 = op_93

    // STY $nn,X
    fn step_op_94<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        self.store(sys, self.base1, self.y)
    }

    // STA $nn,X
    fn step_op_95<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        self.store(sys, self.base1, self.a)
    }

    // STX $nn,Y
    fn step_op_96<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.y)?;
        }

        // op_step == 3
        self.store(sys, self.base1, self.x)
    }

    // step_op_97 = op_97
    // step_op_98 = op_98

    // STA $nnnn,Y
    fn step_op_99<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.y, true)?;
        }

        // op_step == 4
        self.store(sys, self.base1, self.a)
    }

    // step_op_9A = op_9A
    // step_op_9B = op_9B

    // SHY $nnnn,X
    fn step_op_9C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        self.store(sys, self.base1, MachineInt(0))
    }

    // STA $nnnn,X
    fn step_op_9D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, true)?;
        }

        // op_step == 4
        self.store(sys, self.base1, self.a)
    }

    // STZ $nnnn,X
    fn step_op_9E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, true)?;
        }

        // op_step == 4
        self.store(sys, self.base1, MachineInt(0))
    }

    // step_op_9F = op_9F
    // step_op_A0 = op_A0

    // LDA ($nn,X)
    fn step_op_A1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izx(sys)?;
        }

        // op_step == 5
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // step_op_A2 = op_A2
    // step_op_A3 = op_A3

    // LDY $nn
    fn step_op_A4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nn
    fn step_op_A5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nn
    fn step_op_A6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // step_op_A7 = op_A7
    // step_op_A8 = op_A8
    // step_op_A9 = op_A9
    // step_op_AA = op_AA
    // step_op_AB = op_AB

    // LDY $nnnn
    fn step_op_AC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nnnn
    fn step_op_AD<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nnnn
    fn step_op_AE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // step_op_AF = op_AF

    // BCS
    fn step_op_B0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.step_branch(sys, self.flags.c())
    }

    // LDA ($nn),Y
    fn step_op_B1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izy(sys, false)?;
        }

        // op_step == 5
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDA ($nn)
    fn step_op_B2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_izp(sys)?;
        }

        // op_step == 4
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // step_op_B3 = op_B3

    // LDY $nn,X
    fn step_op_B4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nn,X
    fn step_op_B5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nn,Y
    fn step_op_B6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.y)?;
        }

        // op_step == 3
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // step_op_B7 = op_B7
    // step_op_B8 = op_B8

    // LDA $nnnn,Y
    fn step_op_B9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.y, false)?;
        }

        // op_step == 4
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // step_op_BA = op_BA
    // step_op_BB = op_BB

    // LDY $nnnn,X
    fn step_op_BC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step == 4
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nnnn,X
    fn step_op_BD<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step == 4
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nnnn,Y
    fn step_op_BE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.y, false)?;
        }

        // op_step == 4
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // step_op_BF = op_BF
    // step_op_C0 = op_C0

    // CMP ($nn,X)
    fn step_op_C1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izx(sys)?;
        }

        // op_step == 5
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // step_op_C2 = op_C2
    // step_op_C3 = op_C3

    // CPY $nn
    fn step_op_C4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        let val = self.load(sys, self.base1)?;
        self.CMP(self.y, val);
        Some(())
    }

    // CMP $nn
    fn step_op_C5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nn
    fn step_op_C6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step >= 2
        self.step_rmw(sys, self.base1, Cmos::DEC, 2)
    }

    // step_op_C7 = op_C7
    // step_op_C8 = op_C8
    // step_op_C9 = op_C9
    // step_op_CA = op_CA
    // step_op_CB = op_CB

    // CPY $nnnn
    fn step_op_CC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.CMP(self.y, val);
        Some(())
    }

    // CMP $nnnn
    fn step_op_CD<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nnnn
    fn step_op_CE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::DEC, 3)
    }

    // step_op_CF = op_CF

    // BNE
    fn step_op_D0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.step_branch(sys, !self.flags.z())
    }

    // CMP ($nn),Y
    fn step_op_D1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izy(sys, false)?;
        }

        // op_step == 5
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // CMP ($nn)
    fn step_op_D2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_izp(sys)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // step_op_D3 = op_D3

    // NOP $nn,X
    fn step_op_D4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        self.load(sys, self.base1)?;
        Some(())
    }

    // CMP $nn,X
    fn step_op_D5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nn,X
    fn step_op_D6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::DEC, 3)
    }

    // step_op_D7 = op_D7
    // step_op_D8 = op_D8

    // CMP $nnnn,Y
    fn step_op_D9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.y, false)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // PHX
    fn step_op_DA<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.read(sys, self.pc)?;
        }

        // op_step == 2
        self.store(sys, Addr::stack(self.sp), self.x)?;
        self.sp -= 1;
        Some(())
    }

    // step_op_DB = op_DB

    // NOP $nnnn,X (4-cycle)
    fn step_op_DC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 4
        self.load(sys, self.base1.no_carry(self.x))?;
        Some(())
    }

    // CMP $nnnn,X
    fn step_op_DD<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step == 4
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nnnn,X
    fn step_op_DE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, true)?;
        }

        // op_step >= 4
        self.step_rmw(sys, self.base1, Cmos::DEC, 4)
    }

    // step_op_DF = op_DF
    // step_op_E0 = op_E0

    // SBC ($nn,X)
    fn step_op_E1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izx(sys)?;
        }

        // op_step >= 5
        self.step_decimal(sys, self.base1, Cmos::SBC, 5)
    }

    // step_op_E2 = op_E2
    // step_op_E3 = op_E3

    // CPX $nn
    fn step_op_E4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step == 2
        let val = self.load(sys, self.base1)?;
        self.CMP(self.x, val);
        Some(())
    }

    // SBC $nn
    fn step_op_E5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step >= 2
        self.step_decimal(sys, self.base1, Cmos::SBC, 2)
    }

    // INC $nn
    fn step_op_E6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_step >= 2
        self.step_rmw(sys, self.base1, Cmos::INC, 2)
    }

    // step_op_E7 = op_E7
    // step_op_E8 = op_E8

    fn step_op_E9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            if !self.flags.d {
                self.poll_signals(sys);
            }
            self.lo_byte = self.fetch_operand(sys)?;
        }

        // op_step == 2
        if self.flags.d {
            self.load(sys, self.pc)?;
        }
        self.SBC(self.lo_byte);
        Some(())
    }

    // step_op_EA = op_EA
    // step_op_EB = op_EB

    // CPX $nnnn
    fn step_op_EC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 3
        let val = self.load(sys, self.base1)?;
        self.CMP(self.x, val);
        Some(())
    }

    // SBC $nnnn
    fn step_op_ED<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step >= 3
        self.step_decimal(sys, self.base1, Cmos::SBC, 3)
    }

    // INC $nnnn
    fn step_op_EE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::INC, 3)
    }

    // step_op_EF = op_EF

    // BEQ
    fn step_op_F0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.step_branch(sys, self.flags.z())
    }

    // SBC ($nn),Y
    fn step_op_F1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 5 {
            self.base1 = self.step_addr_izy(sys, false)?;
        }

        // op_step >= 5
        self.step_decimal(sys, self.base1, Cmos::SBC, 5)
    }

    // SBC ($nn)
    fn step_op_F2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_izp(sys)?;
        }

        // op_step >= 4
        self.step_decimal(sys, self.base1, Cmos::SBC, 4)
    }

    // step_op_F3 = op_F3

    // NOP $nn,X
    fn step_op_F4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step == 3
        self.load(sys, self.base1)?;
        Some(())
    }

    // SBC $nn,X
    fn step_op_F5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step >= 3
        self.step_decimal(sys, self.base1, Cmos::SBC, 3)
    }

    // INC $nn,X
    fn step_op_F6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_zpi(sys, self.x)?;
        }

        // op_step >= 3
        self.step_rmw(sys, self.base1, Cmos::INC, 3)
    }

    // step_op_F7 = op_F7
    // step_op_F8 = op_F8

    // SBC $nnnn,Y
    fn step_op_F9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.y, false)?;
        }

        // op_step >= 4
        self.step_decimal(sys, self.base1, Cmos::SBC, 4)
    }

    // PLX
    fn step_op_FA<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step == 1 {
            self.read(sys, self.pc)?;
        }

        if self.op_step == 2 {
            self.read_stack(sys)?;
            self.sp += 1;
        }

        // op_step == 3
        self.x = self.load(sys, Addr::stack(self.sp))?;
        self.flags.nz(self.x);
        Some(())
    }

    // step_op_FB = op_FB

    // NOP $nnnn,X
    fn step_op_FC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 3 {
            self.base1 = self.step_addr_abs(sys)?;
        }

        // op_step == 4
        self.load(sys, self.base1.no_carry(self.x))?;
        Some(())
    }

    // SBC $nnnn,X
    fn step_op_FD<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, false)?;
        }

        // op_step >= 4
        self.step_decimal(sys, self.base1, Cmos::SBC, 4)
    }

    // INC $nnnn,X
    fn step_op_FE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_step < 4 {
            self.base1 = self.step_addr_abi(sys, self.x, true)?;
        }

        // op_step >= 4
        self.step_rmw(sys, self.base1, Cmos::INC, 4)
    }

    // step_op_FF = op_FF
}

impl Cmos {
    #[cfg_attr(
        feature = "cargo-clippy",
        allow(clippy::cyclomatic_complexity)
    )]
    pub(crate) fn step_exec<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        match self.op {
            0x00 => self.step_op_00(sys)?,
            0x01 => self.step_op_01(sys)?,
            0x02 => self.op_02(sys)?,
            0x03 => self.op_03(sys)?,
            0x04 => self.step_op_04(sys)?,
            0x05 => self.step_op_05(sys)?,
            0x06 => self.step_op_06(sys)?,
            0x07 => self.op_07(sys)?,
            0x08 => self.step_op_08(sys)?,
            0x09 => self.op_09(sys)?,
            0x0a => self.op_0A(sys)?,
            0x0b => self.op_0B(sys)?,
            0x0c => self.step_op_0C(sys)?,
            0x0d => self.step_op_0D(sys)?,
            0x0e => self.step_op_0E(sys)?,
            0x0f => self.op_0F(sys)?,
            0x10 => self.step_op_10(sys)?,
            0x11 => self.step_op_11(sys)?,
            0x12 => self.step_op_12(sys)?,
            0x13 => self.op_13(sys)?,
            0x14 => self.step_op_14(sys)?,
            0x15 => self.step_op_15(sys)?,
            0x16 => self.step_op_16(sys)?,
            0x17 => self.op_17(sys)?,
            0x18 => self.op_18(sys)?,
            0x19 => self.step_op_19(sys)?,
            0x1a => self.op_1A(sys)?,
            0x1b => self.op_1B(sys)?,
            0x1c => self.step_op_1C(sys)?,
            0x1d => self.step_op_1D(sys)?,
            0x1e => self.step_op_1E(sys)?,
            0x1f => self.op_1F(sys)?,
            0x20 => self.step_op_20(sys)?,
            0x21 => self.step_op_21(sys)?,
            0x22 => self.op_22(sys)?,
            0x23 => self.op_23(sys)?,
            0x24 => self.step_op_24(sys)?,
            0x25 => self.step_op_25(sys)?,
            0x26 => self.step_op_26(sys)?,
            0x27 => self.op_27(sys)?,
            0x28 => self.step_op_28(sys)?,
            0x29 => self.op_29(sys)?,
            0x2a => self.op_2A(sys)?,
            0x2b => self.op_2B(sys)?,
            0x2c => self.step_op_2C(sys)?,
            0x2d => self.step_op_2D(sys)?,
            0x2e => self.step_op_2E(sys)?,
            0x2f => self.op_2F(sys)?,
            0x30 => self.step_op_30(sys)?,
            0x31 => self.step_op_31(sys)?,
            0x32 => self.step_op_32(sys)?,
            0x33 => self.op_33(sys)?,
            0x34 => self.step_op_34(sys)?,
            0x35 => self.step_op_35(sys)?,
            0x36 => self.step_op_36(sys)?,
            0x37 => self.op_37(sys)?,
            0x38 => self.op_38(sys)?,
            0x39 => self.step_op_39(sys)?,
            0x3a => self.op_3A(sys)?,
            0x3b => self.op_3B(sys)?,
            0x3c => self.step_op_3C(sys)?,
            0x3d => self.step_op_3D(sys)?,
            0x3e => self.step_op_3E(sys)?,
            0x3f => self.op_3F(sys)?,
            0x40 => self.step_op_40(sys)?,
            0x41 => self.step_op_41(sys)?,
            0x42 => self.op_42(sys)?,
            0x43 => self.op_43(sys)?,
            0x44 => self.step_op_44(sys)?,
            0x45 => self.step_op_45(sys)?,
            0x46 => self.step_op_46(sys)?,
            0x47 => self.op_47(sys)?,
            0x48 => self.step_op_48(sys)?,
            0x49 => self.op_49(sys)?,
            0x4a => self.op_4A(sys)?,
            0x4b => self.op_4B(sys)?,
            0x4c => self.step_op_4C(sys)?,
            0x4d => self.step_op_4D(sys)?,
            0x4e => self.step_op_4E(sys)?,
            0x4f => self.op_4F(sys)?,
            0x50 => self.step_op_50(sys)?,
            0x51 => self.step_op_51(sys)?,
            0x52 => self.step_op_52(sys)?,
            0x53 => self.op_53(sys)?,
            0x54 => self.step_op_54(sys)?,
            0x55 => self.step_op_55(sys)?,
            0x56 => self.step_op_56(sys)?,
            0x57 => self.op_57(sys)?,
            0x58 => self.op_58(sys)?,
            0x59 => self.step_op_59(sys)?,
            0x5a => self.step_op_5A(sys)?,
            0x5b => self.op_5B(sys)?,
            0x5c => self.step_op_5C(sys)?,
            0x5d => self.step_op_5D(sys)?,
            0x5e => self.step_op_5E(sys)?,
            0x5f => self.op_5F(sys)?,
            0x60 => self.step_op_60(sys)?,
            0x61 => self.step_op_61(sys)?,
            0x62 => self.op_62(sys)?,
            0x63 => self.op_63(sys)?,
            0x64 => self.step_op_64(sys)?,
            0x65 => self.step_op_65(sys)?,
            0x66 => self.step_op_66(sys)?,
            0x67 => self.op_67(sys)?,
            0x68 => self.step_op_68(sys)?,
            0x69 => self.step_op_69(sys)?,
            0x6a => self.op_6A(sys)?,
            0x6b => self.op_6B(sys)?,
            0x6c => self.step_op_6C(sys)?,
            0x6d => self.step_op_6D(sys)?,
            0x6e => self.step_op_6E(sys)?,
            0x6f => self.op_6F(sys)?,
            0x70 => self.step_op_70(sys)?,
            0x71 => self.step_op_71(sys)?,
            0x72 => self.step_op_72(sys)?,
            0x73 => self.op_73(sys)?,
            0x74 => self.step_op_74(sys)?,
            0x75 => self.step_op_75(sys)?,
            0x76 => self.step_op_76(sys)?,
            0x77 => self.op_77(sys)?,
            0x78 => self.op_78(sys)?,
            0x79 => self.step_op_79(sys)?,
            0x7a => self.step_op_7A(sys)?,
            0x7b => self.op_7B(sys)?,
            0x7c => self.step_op_7C(sys)?,
            0x7d => self.step_op_7D(sys)?,
            0x7e => self.step_op_7E(sys)?,
            0x7f => self.op_7F(sys)?,
            0x80 => self.step_op_80(sys)?,
            0x81 => self.step_op_81(sys)?,
            0x82 => self.op_82(sys)?,
            0x83 => self.op_83(sys)?,
            0x84 => self.step_op_84(sys)?,
            0x85 => self.step_op_85(sys)?,
            0x86 => self.step_op_86(sys)?,
            0x87 => self.op_87(sys)?,
            0x88 => self.op_88(sys)?,
            0x89 => self.op_89(sys)?,
            0x8a => self.op_8A(sys)?,
            0x8b => self.op_8B(sys)?,
            0x8c => self.step_op_8C(sys)?,
            0x8d => self.step_op_8D(sys)?,
            0x8e => self.step_op_8E(sys)?,
            0x8f => self.op_8F(sys)?,
            0x90 => self.step_op_90(sys)?,
            0x91 => self.step_op_91(sys)?,
            0x92 => self.step_op_92(sys)?,
            0x93 => self.op_93(sys)?,
            0x94 => self.step_op_94(sys)?,
            0x95 => self.step_op_95(sys)?,
            0x96 => self.step_op_96(sys)?,
            0x97 => self.op_97(sys)?,
            0x98 => self.op_98(sys)?,
            0x99 => self.step_op_99(sys)?,
            0x9a => self.op_9A(sys)?,
            0x9b => self.op_9B(sys)?,
            0x9c => self.step_op_9C(sys)?,
            0x9d => self.step_op_9D(sys)?,
            0x9e => self.step_op_9E(sys)?,
            0x9f => self.op_9F(sys)?,
            0xa0 => self.op_A0(sys)?,
            0xa1 => self.step_op_A1(sys)?,
            0xa2 => self.op_A2(sys)?,
            0xa3 => self.op_A3(sys)?,
            0xa4 => self.step_op_A4(sys)?,
            0xa5 => self.step_op_A5(sys)?,
            0xa6 => self.step_op_A6(sys)?,
            0xa7 => self.op_A7(sys)?,
            0xa8 => self.op_A8(sys)?,
            0xa9 => self.op_A9(sys)?,
            0xaa => self.op_AA(sys)?,
            0xab => self.op_AB(sys)?,
            0xac => self.step_op_AC(sys)?,
            0xad => self.step_op_AD(sys)?,
            0xae => self.step_op_AE(sys)?,
            0xaf => self.op_AF(sys)?,
            0xb0 => self.step_op_B0(sys)?,
            0xb1 => self.step_op_B1(sys)?,
            0xb2 => self.step_op_B2(sys)?,
            0xb3 => self.op_B3(sys)?,
            0xb4 => self.step_op_B4(sys)?,
            0xb5 => self.step_op_B5(sys)?,
            0xb6 => self.step_op_B6(sys)?,
            0xb7 => self.op_B7(sys)?,
            0xb8 => self.op_B8(sys)?,
            0xb9 => self.step_op_B9(sys)?,
            0xba => self.op_BA(sys)?,
            0xbb => self.op_BB(sys)?,
            0xbc => self.step_op_BC(sys)?,
            0xbd => self.step_op_BD(sys)?,
            0xbe => self.step_op_BE(sys)?,
            0xbf => self.op_BF(sys)?,
            0xc0 => self.op_C0(sys)?,
            0xc1 => self.step_op_C1(sys)?,
            0xc2 => self.op_C2(sys)?,
            0xc3 => self.op_C3(sys)?,
            0xc4 => self.step_op_C4(sys)?,
            0xc5 => self.step_op_C5(sys)?,
            0xc6 => self.step_op_C6(sys)?,
            0xc7 => self.op_C7(sys)?,
            0xc8 => self.op_C8(sys)?,
            0xc9 => self.op_C9(sys)?,
            0xca => self.op_CA(sys)?,
            0xcb => self.op_CB(sys)?,
            0xcc => self.step_op_CC(sys)?,
            0xcd => self.step_op_CD(sys)?,
            0xce => self.step_op_CE(sys)?,
            0xcf => self.op_CF(sys)?,
            0xd0 => self.step_op_D0(sys)?,
            0xd1 => self.step_op_D1(sys)?,
            0xd2 => self.step_op_D2(sys)?,
            0xd3 => self.op_D3(sys)?,
            0xd4 => self.step_op_D4(sys)?,
            0xd5 => self.step_op_D5(sys)?,
            0xd6 => self.step_op_D6(sys)?,
            0xd7 => self.op_D7(sys)?,
            0xd8 => self.op_D8(sys)?,
            0xd9 => self.step_op_D9(sys)?,
            0xda => self.step_op_DA(sys)?,
            0xdb => self.op_DB(sys)?,
            0xdc => self.step_op_DC(sys)?,
            0xdd => self.step_op_DD(sys)?,
            0xde => self.step_op_DE(sys)?,
            0xdf => self.op_DF(sys)?,
            0xe0 => self.op_E0(sys)?,
            0xe1 => self.step_op_E1(sys)?,
            0xe2 => self.op_E2(sys)?,
            0xe3 => self.op_E3(sys)?,
            0xe4 => self.step_op_E4(sys)?,
            0xe5 => self.step_op_E5(sys)?,
            0xe6 => self.step_op_E6(sys)?,
            0xe7 => self.op_E7(sys)?,
            0xe8 => self.op_E8(sys)?,
            0xe9 => self.step_op_E9(sys)?,
            0xea => self.op_EA(sys)?,
            0xeb => self.op_EB(sys)?,
            0xec => self.step_op_EC(sys)?,
            0xed => self.step_op_ED(sys)?,
            0xee => self.step_op_EE(sys)?,
            0xef => self.op_EF(sys)?,
            0xf0 => self.step_op_F0(sys)?,
            0xf1 => self.step_op_F1(sys)?,
            0xf2 => self.step_op_F2(sys)?,
            0xf3 => self.op_F3(sys)?,
            0xf4 => self.step_op_F4(sys)?,
            0xf5 => self.step_op_F5(sys)?,
            0xf6 => self.step_op_F6(sys)?,
            0xf7 => self.op_F7(sys)?,
            0xf8 => self.op_F8(sys)?,
            0xf9 => self.step_op_F9(sys)?,
            0xfa => self.step_op_FA(sys)?,
            0xfb => self.op_FB(sys)?,
            0xfc => self.step_op_FC(sys)?,
            0xfd => self.step_op_FD(sys)?,
            0xfe => self.step_op_FE(sys)?,
            0xff => self.op_FF(sys)?,
            _ => unreachable!(),
        }
        Some(())
    }
}
