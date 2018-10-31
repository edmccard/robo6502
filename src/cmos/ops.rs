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

mod cycle;

impl Cmos {
    // BRK
    fn op_00<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        // PC is incremented for BRK but not NMI/IRQ
        if self.do_int {
            self.read(sys, self.pc)?;
        } else {
            self.fetch_operand(sys)?;
        }

        // The stack writes become reads for RES
        if self.reset {
            self.read_stack(sys)?;
        } else {
            self.write_stack(sys, self.pc.hi())?;
        }
        self.sp -= 1;

        if self.reset {
            self.read_stack(sys)?;
        } else {
            self.write_stack(sys, self.pc.lo())?;
        }
        self.sp -= 1;

        self.base1 = self.signal_vector(sys);

        if self.reset {
            self.read_stack(sys)?;
        } else if self.do_int {
            // Clear B flag in saved status for NMI/IRQ
            self.write_stack(sys, self.flags.to_byte() & 0b1110_1111)?;
        } else {
            self.write_stack(sys, self.flags.to_byte())?;
        }
        self.sp -= 1;

        self.lo_byte = self.read(sys, self.base1)?;
        self.hi_byte = self.read(sys, self.base1 + 1)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);

        self.flags.i = true;
        self.flags.d = false;
        self.clear_signals();

        Some(())
    }

    // ORA ($nn,X)
    fn op_01<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izx(sys)?;
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // NOP #nn
    fn op_02<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.immediate(sys)?;
        Some(())
    }

    // NOP (single-cycle)
    fn op_03<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // TSB $nn
    fn op_04<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.rmw(sys, self.base1, Cmos::TSB)
    }

    // ORA $nn
    fn op_05<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nn
    fn op_06<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.rmw(sys, self.base1, Cmos::ASL)
    }

    // NOP (single-cycle)
    fn op_07<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // PHP
    fn op_08<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.store(sys, Addr::stack(self.sp), self.flags.to_byte())?;
        self.sp -= 1;
        Some(())
    }

    // ORA #nn
    fn op_09<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        let val = self.immediate(sys)?;
        self.ORA(val);
        Some(())
    }

    // ASL A
    fn op_0a<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.ASL(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_0b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // TSB $nnnn
    fn op_0c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::TSB)
    }

    // ORA $nnnn
    fn op_0d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nnnn
    fn op_0e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::ASL)
    }

    // NOP (single-cycle)
    fn op_0f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BPL
    fn op_10<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, !self.flags.n())
    }

    // ORA ($nn),Y
    fn op_11<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izy(sys, false)?;
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ORA ($nn)
    fn op_12<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // NOP (single-cycle)
    fn op_13<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // TRB $nn
    fn op_14<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.rmw(sys, self.base1, Cmos::TRB)
    }

    // ORA $nn,X
    fn op_15<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nn,X
    fn op_16<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.rmw(sys, self.base1, Cmos::ASL)
    }

    // NOP (single-cycle)
    fn op_17<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CLC
    fn op_18<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.flags.set_c(false);
        Some(())
    }

    // ORA $nnnn,Y
    fn op_19<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // INC A
    fn op_1a<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.INC(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_1b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // TRB $nnnn
    fn op_1c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::TRB)
    }

    // ORA $nnnn,X
    fn op_1d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nnnn,X
    fn op_1e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        // One cycle less if no px
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.rmw(sys, self.base1, Cmos::ASL)
    }

    // NOP (single-cycle)
    fn op_1f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // JSR $nnnn
    fn op_20<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.lo_byte = self.fetch_operand(sys)?;
        self.read_stack(sys)?;
        self.write_stack(sys, self.pc.hi())?;
        self.sp -= 1;
        self.write_stack(sys, self.pc.lo())?;
        self.sp -= 1;
        self.poll_signals(sys);
        self.hi_byte = self.fetch_operand(sys)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // AND ($nn,X)
    fn op_21<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izx(sys)?;
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // NOP #nn
    fn op_22<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.immediate(sys)?;
        Some(())
    }

    // NOP (single-cycle)
    fn op_23<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BIT $nn
    fn op_24<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nn
    fn op_25<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nn
    fn op_26<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.rmw(sys, self.base1, Cmos::ROL)
    }

    // NOP (single-cycle)
    fn op_27<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // PLP
    fn op_28<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.read_stack(sys)?;
        self.sp += 1;
        let p = self.load(sys, Addr::stack(self.sp))?;
        self.flags.from_byte(p);
        Some(())
    }

    // AND #nn
    fn op_29<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        let val = self.immediate(sys)?;
        self.AND(val);
        Some(())
    }

    // ROL A
    fn op_2a<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.ROL(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_2b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BIT $nnnn
    fn op_2c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nnnn
    fn op_2d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nnnn
    fn op_2e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::ROL)
    }

    // NOP (single-cycle)
    fn op_2f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BMI
    fn op_30<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, self.flags.n())
    }

    // AND ($nn),Y
    fn op_31<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izy(sys, false)?;
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // AND ($nn)
    fn op_32<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // NOP (single-cycle)
    fn op_33<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BIT $nn,X
    fn op_34<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nn,X
    fn op_35<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nn,X
    fn op_36<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.rmw(sys, self.base1, Cmos::ROL)
    }

    // NOP (single-cycle)
    fn op_37<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // SEC
    fn op_38<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.flags.set_c(true);
        Some(())
    }

    // AND $nnnn,Y
    fn op_39<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // DEC A
    fn op_3a<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.DEC(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_3b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BIT $nnnn,X
    fn op_3c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nnnn,X
    fn op_3d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nnnn,X
    fn op_3e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        // One cycle less if no px
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.rmw(sys, self.base1, Cmos::ROL)
    }

    // NOP (single-cycle)
    fn op_3f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // RTI
    fn op_40<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.read_stack(sys)?;
        self.sp += 1;
        let p = self.read_stack(sys)?;
        self.sp += 1;
        self.flags.from_byte(p);
        self.lo_byte = self.read_stack(sys)?;
        self.sp += 1;
        self.poll_signals(sys);
        self.hi_byte = self.read_stack(sys)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // EOR ($nn,X)
    fn op_41<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izx(sys)?;
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // NOP #nn
    fn op_42<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.immediate(sys)?;
        Some(())
    }

    // NOP (single-cycle)
    fn op_43<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP $nn
    fn op_44<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.load(sys, self.base1)?;
        Some(())
    }

    // EOR $nn
    fn op_45<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nn
    fn op_46<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.rmw(sys, self.base1, Cmos::LSR)
    }

    // NOP (single-cycle)
    fn op_47<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // PHA
    fn op_48<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.store(sys, Addr::stack(self.sp), self.a)?;
        self.sp -= 1;
        Some(())
    }

    // EOR #nn
    fn op_49<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        let val = self.immediate(sys)?;
        self.EOR(val);
        Some(())
    }

    // LSR A
    fn op_4a<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.LSR(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_4b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // JMP $nnnn
    fn op_4c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.lo_byte = self.fetch_operand(sys)?;
        self.poll_signals(sys);
        self.hi_byte = self.fetch_operand(sys)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // EOR $nnnn
    fn op_4d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nnnn
    fn op_4e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::LSR)
    }

    // NOP (single-cycle)
    fn op_4f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BVC
    fn op_50<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, !self.flags.v())
    }

    // EOR ($nn),Y
    fn op_51<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izy(sys, false)?;
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // EOR ($nn)
    fn op_52<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // NOP (single-cycle)
    fn op_53<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP $nn,X
    fn op_54<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.load(sys, self.base1)?;
        Some(())
    }

    // EOR $nn,X
    fn op_55<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nn,X
    fn op_56<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.rmw(sys, self.base1, Cmos::LSR)
    }

    // NOP (single-cycle)
    fn op_57<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CLI
    fn op_58<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.flags.i = false;
        Some(())
    }

    // EOR $nnnn,Y
    fn op_59<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // PHY
    fn op_5a<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.store(sys, Addr::stack(self.sp), self.y)?;
        self.sp -= 1;
        Some(())
    }

    // NOP (single-cycle)
    fn op_5b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP (eight-cycle)
    fn op_5c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 =
            Addr::from_bytes(self.addr_abs(sys)?.lo(), MachineInt(0xff));
        self.read(sys, self.base1)?;
        self.read(sys, MachineInt(0xffff))?;
        self.read(sys, MachineInt(0xffff))?;
        self.read(sys, MachineInt(0xffff))?;
        self.load(sys, MachineInt(0xffff))?;
        Some(())
    }

    // EOR $nnnn,X
    fn op_5d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nnnn,X
    fn op_5e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        // One cycle less if no px
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.rmw(sys, self.base1, Cmos::LSR)
    }

    // NOP (single-cycle)
    fn op_5f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // RTS
    fn op_60<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.read_stack(sys)?;
        self.sp += 1;
        self.lo_byte = self.read_stack(sys)?;
        self.sp += 1;
        self.hi_byte = self.read_stack(sys)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        self.poll_signals(sys);
        self.fetch_operand(sys)?;
        Some(())
    }

    // ADC ($nn,X)
    fn op_61<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izx(sys)?;
        self.decimal(sys, self.base1, Cmos::ADC)
    }

    // NOP #nn
    fn op_62<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.immediate(sys)?;
        Some(())
    }

    // NOP (single-cycle)
    fn op_63<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // STZ $nn
    fn op_64<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.store(sys, self.base1, MachineInt(0))?;
        Some(())
    }

    // ADC $nn
    fn op_65<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.decimal(sys, self.base1, Cmos::ADC)
    }

    // ROR $nn
    fn op_66<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.rmw(sys, self.base1, Cmos::ROR)
    }

    // NOP (single-cycle)
    fn op_67<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // PLA
    fn op_68<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.read_stack(sys)?;
        self.sp += 1;
        self.a = self.load(sys, Addr::stack(self.sp))?;
        self.flags.nz(self.a);
        Some(())
    }

    // ADC #nn
    fn op_69<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if !self.flags.d {
            self.poll_signals(sys);
        }
        self.lo_byte = self.fetch_operand(sys)?;
        if self.flags.d {
            self.load(sys, self.pc)?;
        }
        self.ADC(self.lo_byte);
        Some(())
    }

    // ROR A
    fn op_6a<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.ROR(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_6b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // JMP ($nnnn)
    fn op_6c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.lo_byte = self.read(sys, self.base1)?;
        // CMOS: the vector can cross a page.
        self.hi_byte = self.load(sys, self.base1 + 1)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // ADC $nnnn
    fn op_6d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.decimal(sys, self.base1, Cmos::ADC)
    }

    // ROR $nnnn
    fn op_6e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::ROR)
    }

    // NOP (single-cycle)
    fn op_6f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BVS
    fn op_70<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, self.flags.v())
    }

    // ADC ($nn),Y
    fn op_71<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izy(sys, false)?;
        self.decimal(sys, self.base1, Cmos::ADC)
    }

    // ADC ($nn)
    fn op_72<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izp(sys)?;
        self.decimal(sys, self.base1, Cmos::ADC)
    }

    // NOP (single-cycle)
    fn op_73<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // STZ $nn,X
    fn op_74<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.store(sys, self.base1, MachineInt(0))?;
        Some(())
    }

    // ADC $nn,X
    fn op_75<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.decimal(sys, self.base1, Cmos::ADC)
    }

    // ROR $nn,X
    fn op_76<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.rmw(sys, self.base1, Cmos::ROR)
    }

    // NOP (single-cycle)
    fn op_77<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // SEI
    fn op_78<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.flags.i = true;
        Some(())
    }

    // ADC $nnnn,Y
    fn op_79<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        self.decimal(sys, self.base1, Cmos::ADC)
    }

    // PLY
    fn op_7a<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.read_stack(sys)?;
        self.sp += 1;
        self.y = self.load(sys, Addr::stack(self.sp))?;
        self.flags.nz(self.y);
        Some(())
    }

    // NOP (single-cycle)
    fn op_7b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // JMP ($nnnn,X)
    fn op_7c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.lo_byte = self.fetch_operand(sys)?;
        self.hi_byte = self.read(sys, self.pc)?;
        self.base1 = self.addr() + self.x;
        self.fetch_operand(sys)?;
        self.lo_byte = self.read(sys, self.base1)?;
        self.hi_byte = self.load(sys, self.base1 + 1)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // ADC $nnnn,X
    fn op_7d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.decimal(sys, self.base1, Cmos::ADC)
    }

    // ROR $nnnn,X
    fn op_7e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.rmw(sys, self.base1, Cmos::ROR)
    }

    // NOP (single-cycle)
    fn op_7f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BRA
    fn op_80<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, true)
    }

    // STA ($nn,X)
    fn op_81<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izx(sys)?;
        self.store(sys, self.base1, self.a)
    }

    // NOP #nn
    fn op_82<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.immediate(sys)?;
        Some(())
    }

    // NOP (single-cycle)
    fn op_83<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // STY $nn
    fn op_84<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.store(sys, self.base1, self.y)
    }

    // STA $nn
    fn op_85<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.store(sys, self.base1, self.a)
    }

    // STX $nn
    fn op_86<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.store(sys, self.base1, self.x)
    }

    // NOP (single-cycle)
    fn op_87<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // DEY
    fn op_88<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.y -= 1;
        self.flags.nz(self.y);
        Some(())
    }

    // BIT #nn
    fn op_89<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        let val = self.immediate(sys)?;
        self.flags.z = self.a & val;
        Some(())
    }

    // TXA
    fn op_8a<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.x;
        self.flags.nz(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_8b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // STY $nnnn
    fn op_8c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.store(sys, self.base1, self.y)
    }

    // STA $nnnn
    fn op_8d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.store(sys, self.base1, self.a)
    }

    // STX $nnnn
    fn op_8e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.store(sys, self.base1, self.x)
    }

    // NOP (single-cycle)
    fn op_8f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BCC
    fn op_90<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, !self.flags.c())
    }

    // STA ($nn),Y
    fn op_91<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izy(sys, true)?;
        self.store(sys, self.base1, self.a)
    }

    // STA ($nn)
    fn op_92<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izp(sys)?;
        self.store(sys, self.base1, self.a)
    }

    // NOP (single-cycle)
    fn op_93<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // STY $nn,X
    fn op_94<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.store(sys, self.base1, self.y)
    }

    // STA $nn,X
    fn op_95<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.store(sys, self.base1, self.a)
    }

    // STX $nn,Y
    fn op_96<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.y)?;
        self.store(sys, self.base1, self.x)
    }

    // NOP (single-cycle)
    fn op_97<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // TYA
    fn op_98<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.y;
        self.flags.nz(self.a);
        Some(())
    }

    // STA $nnnn,Y
    fn op_99<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, true)?;
        self.store(sys, self.base1, self.a)
    }

    // TXS
    fn op_9a<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.sp = self.x;
        Some(())
    }

    // NOP (single-cycle)
    fn op_9b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // STZ $nnnn
    fn op_9c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.store(sys, self.base1, MachineInt(0))
    }

    // STA $nnnn,X
    fn op_9d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, true)?;
        self.store(sys, self.base1, self.a)
    }

    // STZ $nnnn,X
    fn op_9e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, true)?;
        self.store(sys, self.base1, MachineInt(0))
    }

    // NOP (single-cycle)
    fn op_9f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // LDY #nn
    fn op_a0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.y = self.immediate(sys)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA ($nn,X)
    fn op_a1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izx(sys)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX #nn
    fn op_a2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.x = self.immediate(sys)?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_a3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // LDY $nn
    fn op_a4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nn
    fn op_a5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nn
    fn op_a6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_a7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // TAY
    fn op_a8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.y = self.a;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA #nn
    fn op_a9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.a = self.immediate(sys)?;
        self.flags.nz(self.a);
        Some(())
    }

    // TAX
    fn op_aa<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.x = self.a;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_ab<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // LDY $nnnn
    fn op_ac<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nnnn
    fn op_ad<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nnnn
    fn op_ae<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_af<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BCS
    fn op_b0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, self.flags.c())
    }

    // LDA ($nn),Y
    fn op_b1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izy(sys, false)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDA ($nn)
    fn op_b2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izp(sys)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_b3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // LDY $nn,X
    fn op_b4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nn,X
    fn op_b5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nn,Y
    fn op_b6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.y)?;
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_b7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CLV
    fn op_b8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.flags.set_v(false);
        Some(())
    }

    // LDA $nnnn,Y
    fn op_b9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // TSX
    fn op_ba<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.x = self.sp;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_bb<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // LDY $nnnn,X
    fn op_bc<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nnnn,X
    fn op_bd<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nnnn,Y
    fn op_be<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_bf<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPY #nn
    fn op_c0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        let val = self.immediate(sys)?;
        self.CMP(self.y, val);
        Some(())
    }

    // CMP ($nn,X)
    fn op_c1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izx(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // NOP #nn
    fn op_c2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.immediate(sys)?;
        Some(())
    }

    // NOP (single-cycle)
    fn op_c3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPY $nn
    fn op_c4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.y, val);
        Some(())
    }

    // CMP $nn
    fn op_c5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nn
    fn op_c6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.rmw(sys, self.base1, Cmos::DEC)
    }

    // NOP (single-cycle)
    fn op_c7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // INY
    fn op_c8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.y += 1;
        self.flags.nz(self.y);
        Some(())
    }

    // CMP #nn
    fn op_c9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        let val = self.immediate(sys)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEX
    fn op_ca<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.x -= 1;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_cb<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPY $nnnn
    fn op_cc<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.y, val);
        Some(())
    }

    // CMP $nnnn
    fn op_cd<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nnnn
    fn op_ce<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::DEC)
    }

    // NOP (single-cycle)
    fn op_cf<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BNE
    fn op_d0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, !self.flags.z())
    }

    // CMP ($nn),Y
    fn op_d1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izy(sys, false)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // CMP ($nn)
    fn op_d2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // NOP (single-cycle)
    fn op_d3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP $nn,X
    fn op_d4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.load(sys, self.base1)?;
        Some(())
    }

    // CMP $nn,X
    fn op_d5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nn,X
    fn op_d6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.rmw(sys, self.base1, Cmos::DEC)
    }

    // NOP (single-cycle)
    fn op_d7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CLD
    fn op_d8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.flags.d = false;
        Some(())
    }

    // CMP $nnnn,Y
    fn op_d9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // PHX
    fn op_da<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.store(sys, Addr::stack(self.sp), self.x)?;
        self.sp -= 1;
        Some(())
    }

    // NOP (single-cycle)
    fn op_db<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP $nnnn,X (4-cycle)
    fn op_dc<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.load(sys, self.base1.no_carry(self.x))?;
        Some(())
    }

    // CMP $nnnn,X
    fn op_dd<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nnnn,X
    fn op_de<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, true)?;
        self.rmw(sys, self.base1, Cmos::DEC)
    }

    // NOP (single-cycle)
    fn op_df<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPX #nn
    fn op_e0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        let val = self.immediate(sys)?;
        self.CMP(self.x, val);
        Some(())
    }

    // SBC ($nn,X)
    fn op_e1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izx(sys)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // NOP #nn
    fn op_e2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.immediate(sys)?;
        Some(())
    }

    // NOP (single-cycle)
    fn op_e3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPX $nn
    fn op_e4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.x, val);
        Some(())
    }

    // SBC $nn
    fn op_e5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // INC $nn
    fn op_e6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.rmw(sys, self.base1, Cmos::INC)
    }

    // NOP (single-cycle)
    fn op_e7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // INX
    fn op_e8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.x += 1;
        self.flags.nz(self.x);
        Some(())
    }

    // SBC #nn
    fn op_e9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if !self.flags.d {
            self.poll_signals(sys);
        }
        self.lo_byte = self.fetch_operand(sys)?;
        if self.flags.d {
            self.load(sys, self.pc)?;
        }
        self.SBC(self.lo_byte);
        Some(())
    }

    // NOP
    fn op_ea<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)
    }

    // NOP (single-cycle)
    fn op_eb<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPX $nnnn
    fn op_ec<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.x, val);
        Some(())
    }

    // SBC $nnnn
    fn op_ed<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // INC $nnnn
    fn op_ee<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::INC)
    }

    // NOP (single-cycle)
    fn op_ef<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BEQ
    fn op_f0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, self.flags.z())
    }

    // SBC ($nn),Y
    fn op_f1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izy(sys, false)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // SBC ($nn)
    fn op_f2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izp(sys)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // NOP (single-cycle)
    fn op_f3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP $nn,X
    fn op_f4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.load(sys, self.base1)?;
        Some(())
    }

    // SBC $nn,X
    fn op_f5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // INC $nn,X
    fn op_f6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.rmw(sys, self.base1, Cmos::INC)
    }

    // NOP (single-cycle)
    fn op_f7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // SED
    fn op_f8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.flags.d = true;
        Some(())
    }

    // SBC $nnnn,Y
    fn op_f9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // PLX
    fn op_fa<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.read_stack(sys)?;
        self.sp += 1;
        self.x = self.load(sys, Addr::stack(self.sp))?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_fb<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP $nnnn,X (4-cycle)
    fn op_fc<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.load(sys, self.base1.no_carry(self.x))?;
        Some(())
    }

    // SBC $nnnn,X
    fn op_fd<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // INC $nnnn,X
    fn op_fe<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, true)?;
        self.rmw(sys, self.base1, Cmos::INC)
    }

    // NOP (single-cycle)
    fn op_ff<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }
}

impl Cmos {
    #[cfg_attr(
        feature = "cargo-clippy",
        allow(clippy::cyclomatic_complexity)
    )]
    pub(crate) fn exec<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        match self.op {
            0x00 => self.op_00(sys)?,
            0x01 => self.op_01(sys)?,
            0x02 => self.op_02(sys)?,
            0x03 => self.op_03(sys)?,
            0x04 => self.op_04(sys)?,
            0x05 => self.op_05(sys)?,
            0x06 => self.op_06(sys)?,
            0x07 => self.op_07(sys)?,
            0x08 => self.op_08(sys)?,
            0x09 => self.op_09(sys)?,
            0x0a => self.op_0a(sys)?,
            0x0b => self.op_0b(sys)?,
            0x0c => self.op_0c(sys)?,
            0x0d => self.op_0d(sys)?,
            0x0e => self.op_0e(sys)?,
            0x0f => self.op_0f(sys)?,
            0x10 => self.op_10(sys)?,
            0x11 => self.op_11(sys)?,
            0x12 => self.op_12(sys)?,
            0x13 => self.op_13(sys)?,
            0x14 => self.op_14(sys)?,
            0x15 => self.op_15(sys)?,
            0x16 => self.op_16(sys)?,
            0x17 => self.op_17(sys)?,
            0x18 => self.op_18(sys)?,
            0x19 => self.op_19(sys)?,
            0x1a => self.op_1a(sys)?,
            0x1b => self.op_1b(sys)?,
            0x1c => self.op_1c(sys)?,
            0x1d => self.op_1d(sys)?,
            0x1e => self.op_1e(sys)?,
            0x1f => self.op_1f(sys)?,
            0x20 => self.op_20(sys)?,
            0x21 => self.op_21(sys)?,
            0x22 => self.op_22(sys)?,
            0x23 => self.op_23(sys)?,
            0x24 => self.op_24(sys)?,
            0x25 => self.op_25(sys)?,
            0x26 => self.op_26(sys)?,
            0x27 => self.op_27(sys)?,
            0x28 => self.op_28(sys)?,
            0x29 => self.op_29(sys)?,
            0x2a => self.op_2a(sys)?,
            0x2b => self.op_2b(sys)?,
            0x2c => self.op_2c(sys)?,
            0x2d => self.op_2d(sys)?,
            0x2e => self.op_2e(sys)?,
            0x2f => self.op_2f(sys)?,
            0x30 => self.op_30(sys)?,
            0x31 => self.op_31(sys)?,
            0x32 => self.op_32(sys)?,
            0x33 => self.op_33(sys)?,
            0x34 => self.op_34(sys)?,
            0x35 => self.op_35(sys)?,
            0x36 => self.op_36(sys)?,
            0x37 => self.op_37(sys)?,
            0x38 => self.op_38(sys)?,
            0x39 => self.op_39(sys)?,
            0x3a => self.op_3a(sys)?,
            0x3b => self.op_3b(sys)?,
            0x3c => self.op_3c(sys)?,
            0x3d => self.op_3d(sys)?,
            0x3e => self.op_3e(sys)?,
            0x3f => self.op_3f(sys)?,
            0x40 => self.op_40(sys)?,
            0x41 => self.op_41(sys)?,
            0x42 => self.op_42(sys)?,
            0x43 => self.op_43(sys)?,
            0x44 => self.op_44(sys)?,
            0x45 => self.op_45(sys)?,
            0x46 => self.op_46(sys)?,
            0x47 => self.op_47(sys)?,
            0x48 => self.op_48(sys)?,
            0x49 => self.op_49(sys)?,
            0x4a => self.op_4a(sys)?,
            0x4b => self.op_4b(sys)?,
            0x4c => self.op_4c(sys)?,
            0x4d => self.op_4d(sys)?,
            0x4e => self.op_4e(sys)?,
            0x4f => self.op_4f(sys)?,
            0x50 => self.op_50(sys)?,
            0x51 => self.op_51(sys)?,
            0x52 => self.op_52(sys)?,
            0x53 => self.op_53(sys)?,
            0x54 => self.op_54(sys)?,
            0x55 => self.op_55(sys)?,
            0x56 => self.op_56(sys)?,
            0x57 => self.op_57(sys)?,
            0x58 => self.op_58(sys)?,
            0x59 => self.op_59(sys)?,
            0x5a => self.op_5a(sys)?,
            0x5b => self.op_5b(sys)?,
            0x5c => self.op_5c(sys)?,
            0x5d => self.op_5d(sys)?,
            0x5e => self.op_5e(sys)?,
            0x5f => self.op_5f(sys)?,
            0x60 => self.op_60(sys)?,
            0x61 => self.op_61(sys)?,
            0x62 => self.op_62(sys)?,
            0x63 => self.op_63(sys)?,
            0x64 => self.op_64(sys)?,
            0x65 => self.op_65(sys)?,
            0x66 => self.op_66(sys)?,
            0x67 => self.op_67(sys)?,
            0x68 => self.op_68(sys)?,
            0x69 => self.op_69(sys)?,
            0x6a => self.op_6a(sys)?,
            0x6b => self.op_6b(sys)?,
            0x6c => self.op_6c(sys)?,
            0x6d => self.op_6d(sys)?,
            0x6e => self.op_6e(sys)?,
            0x6f => self.op_6f(sys)?,
            0x70 => self.op_70(sys)?,
            0x71 => self.op_71(sys)?,
            0x72 => self.op_72(sys)?,
            0x73 => self.op_73(sys)?,
            0x74 => self.op_74(sys)?,
            0x75 => self.op_75(sys)?,
            0x76 => self.op_76(sys)?,
            0x77 => self.op_77(sys)?,
            0x78 => self.op_78(sys)?,
            0x79 => self.op_79(sys)?,
            0x7a => self.op_7a(sys)?,
            0x7b => self.op_7b(sys)?,
            0x7c => self.op_7c(sys)?,
            0x7d => self.op_7d(sys)?,
            0x7e => self.op_7e(sys)?,
            0x7f => self.op_7f(sys)?,
            0x80 => self.op_80(sys)?,
            0x81 => self.op_81(sys)?,
            0x82 => self.op_82(sys)?,
            0x83 => self.op_83(sys)?,
            0x84 => self.op_84(sys)?,
            0x85 => self.op_85(sys)?,
            0x86 => self.op_86(sys)?,
            0x87 => self.op_87(sys)?,
            0x88 => self.op_88(sys)?,
            0x89 => self.op_89(sys)?,
            0x8a => self.op_8a(sys)?,
            0x8b => self.op_8b(sys)?,
            0x8c => self.op_8c(sys)?,
            0x8d => self.op_8d(sys)?,
            0x8e => self.op_8e(sys)?,
            0x8f => self.op_8f(sys)?,
            0x90 => self.op_90(sys)?,
            0x91 => self.op_91(sys)?,
            0x92 => self.op_92(sys)?,
            0x93 => self.op_93(sys)?,
            0x94 => self.op_94(sys)?,
            0x95 => self.op_95(sys)?,
            0x96 => self.op_96(sys)?,
            0x97 => self.op_97(sys)?,
            0x98 => self.op_98(sys)?,
            0x99 => self.op_99(sys)?,
            0x9a => self.op_9a(sys)?,
            0x9b => self.op_9b(sys)?,
            0x9c => self.op_9c(sys)?,
            0x9d => self.op_9d(sys)?,
            0x9e => self.op_9e(sys)?,
            0x9f => self.op_9f(sys)?,
            0xa0 => self.op_a0(sys)?,
            0xa1 => self.op_a1(sys)?,
            0xa2 => self.op_a2(sys)?,
            0xa3 => self.op_a3(sys)?,
            0xa4 => self.op_a4(sys)?,
            0xa5 => self.op_a5(sys)?,
            0xa6 => self.op_a6(sys)?,
            0xa7 => self.op_a7(sys)?,
            0xa8 => self.op_a8(sys)?,
            0xa9 => self.op_a9(sys)?,
            0xaa => self.op_aa(sys)?,
            0xab => self.op_ab(sys)?,
            0xac => self.op_ac(sys)?,
            0xad => self.op_ad(sys)?,
            0xae => self.op_ae(sys)?,
            0xaf => self.op_af(sys)?,
            0xb0 => self.op_b0(sys)?,
            0xb1 => self.op_b1(sys)?,
            0xb2 => self.op_b2(sys)?,
            0xb3 => self.op_b3(sys)?,
            0xb4 => self.op_b4(sys)?,
            0xb5 => self.op_b5(sys)?,
            0xb6 => self.op_b6(sys)?,
            0xb7 => self.op_b7(sys)?,
            0xb8 => self.op_b8(sys)?,
            0xb9 => self.op_b9(sys)?,
            0xba => self.op_ba(sys)?,
            0xbb => self.op_bb(sys)?,
            0xbc => self.op_bc(sys)?,
            0xbd => self.op_bd(sys)?,
            0xbe => self.op_be(sys)?,
            0xbf => self.op_bf(sys)?,
            0xc0 => self.op_c0(sys)?,
            0xc1 => self.op_c1(sys)?,
            0xc2 => self.op_c2(sys)?,
            0xc3 => self.op_c3(sys)?,
            0xc4 => self.op_c4(sys)?,
            0xc5 => self.op_c5(sys)?,
            0xc6 => self.op_c6(sys)?,
            0xc7 => self.op_c7(sys)?,
            0xc8 => self.op_c8(sys)?,
            0xc9 => self.op_c9(sys)?,
            0xca => self.op_ca(sys)?,
            0xcb => self.op_cb(sys)?,
            0xcc => self.op_cc(sys)?,
            0xcd => self.op_cd(sys)?,
            0xce => self.op_ce(sys)?,
            0xcf => self.op_cf(sys)?,
            0xd0 => self.op_d0(sys)?,
            0xd1 => self.op_d1(sys)?,
            0xd2 => self.op_d2(sys)?,
            0xd3 => self.op_d3(sys)?,
            0xd4 => self.op_d4(sys)?,
            0xd5 => self.op_d5(sys)?,
            0xd6 => self.op_d6(sys)?,
            0xd7 => self.op_d7(sys)?,
            0xd8 => self.op_d8(sys)?,
            0xd9 => self.op_d9(sys)?,
            0xda => self.op_da(sys)?,
            0xdb => self.op_db(sys)?,
            0xdc => self.op_dc(sys)?,
            0xdd => self.op_dd(sys)?,
            0xde => self.op_de(sys)?,
            0xdf => self.op_df(sys)?,
            0xe0 => self.op_e0(sys)?,
            0xe1 => self.op_e1(sys)?,
            0xe2 => self.op_e2(sys)?,
            0xe3 => self.op_e3(sys)?,
            0xe4 => self.op_e4(sys)?,
            0xe5 => self.op_e5(sys)?,
            0xe6 => self.op_e6(sys)?,
            0xe7 => self.op_e7(sys)?,
            0xe8 => self.op_e8(sys)?,
            0xe9 => self.op_e9(sys)?,
            0xea => self.op_ea(sys)?,
            0xeb => self.op_eb(sys)?,
            0xec => self.op_ec(sys)?,
            0xed => self.op_ed(sys)?,
            0xee => self.op_ee(sys)?,
            0xef => self.op_ef(sys)?,
            0xf0 => self.op_f0(sys)?,
            0xf1 => self.op_f1(sys)?,
            0xf2 => self.op_f2(sys)?,
            0xf3 => self.op_f3(sys)?,
            0xf4 => self.op_f4(sys)?,
            0xf5 => self.op_f5(sys)?,
            0xf6 => self.op_f6(sys)?,
            0xf7 => self.op_f7(sys)?,
            0xf8 => self.op_f8(sys)?,
            0xf9 => self.op_f9(sys)?,
            0xfa => self.op_fa(sys)?,
            0xfb => self.op_fb(sys)?,
            0xfc => self.op_fc(sys)?,
            0xfd => self.op_fd(sys)?,
            0xfe => self.op_fe(sys)?,
            0xff => self.op_ff(sys)?,
            _ => unreachable!(),
        }
        Some(())
    }
}
