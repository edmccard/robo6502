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

mod step;

#[allow(non_snake_case)]
impl Cmos {
    // BRK
    fn op_00<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        // PC is incremented for BRK but not NMI/IRQ
        if self.latch {
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
        } else if self.latch {
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
    fn op_0A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.ASL(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_0B<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // TSB $nnnn
    fn op_0C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::TSB)
    }

    // ORA $nnnn
    fn op_0D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nnnn
    fn op_0E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::ASL)
    }

    // NOP (single-cycle)
    fn op_0F<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_1A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.INC(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_1B<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // TRB $nnnn
    fn op_1C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::TRB)
    }

    // ORA $nnnn,X
    fn op_1D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nnnn,X
    fn op_1E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        // One cycle less if no px
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.rmw(sys, self.base1, Cmos::ASL)
    }

    // NOP (single-cycle)
    fn op_1F<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_2A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.ROL(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_2B<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BIT $nnnn
    fn op_2C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nnnn
    fn op_2D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nnnn
    fn op_2E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::ROL)
    }

    // NOP (single-cycle)
    fn op_2F<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_3A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.DEC(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_3B<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BIT $nnnn,X
    fn op_3C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nnnn,X
    fn op_3D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nnnn,X
    fn op_3E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        // One cycle less if no px
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.rmw(sys, self.base1, Cmos::ROL)
    }

    // NOP (single-cycle)
    fn op_3F<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_4A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.LSR(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_4B<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // JMP $nnnn
    fn op_4C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.lo_byte = self.fetch_operand(sys)?;
        self.poll_signals(sys);
        self.hi_byte = self.fetch_operand(sys)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // EOR $nnnn
    fn op_4D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nnnn
    fn op_4E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::LSR)
    }

    // NOP (single-cycle)
    fn op_4F<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_5A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.store(sys, Addr::stack(self.sp), self.y)?;
        self.sp -= 1;
        Some(())
    }

    // NOP (single-cycle)
    fn op_5B<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP (eight-cycle)
    fn op_5C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_5D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nnnn,X
    fn op_5E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        // One cycle less if no px
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.rmw(sys, self.base1, Cmos::LSR)
    }

    // NOP (single-cycle)
    fn op_5F<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_6A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.ROR(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_6B<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // JMP ($nnnn)
    fn op_6C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.lo_byte = self.read(sys, self.base1)?;
        // CMOS: the vector can cross a page.
        self.hi_byte = self.load(sys, self.base1 + 1)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // ADC $nnnn
    fn op_6D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.decimal(sys, self.base1, Cmos::ADC)
    }

    // ROR $nnnn
    fn op_6E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::ROR)
    }

    // NOP (single-cycle)
    fn op_6F<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_7A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.read_stack(sys)?;
        self.sp += 1;
        self.y = self.load(sys, Addr::stack(self.sp))?;
        self.flags.nz(self.y);
        Some(())
    }

    // NOP (single-cycle)
    fn op_7B<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // JMP ($nnnn,X)
    fn op_7C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_7D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.decimal(sys, self.base1, Cmos::ADC)
    }

    // ROR $nnnn,X
    fn op_7E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.rmw(sys, self.base1, Cmos::ROR)
    }

    // NOP (single-cycle)
    fn op_7F<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_8A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.a = self.x;
        self.flags.nz(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_8B<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // STY $nnnn
    fn op_8C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.store(sys, self.base1, self.y)
    }

    // STA $nnnn
    fn op_8D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.store(sys, self.base1, self.a)
    }

    // STX $nnnn
    fn op_8E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.store(sys, self.base1, self.x)
    }

    // NOP (single-cycle)
    fn op_8F<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_9A<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.sp = self.x;
        Some(())
    }

    // NOP (single-cycle)
    fn op_9B<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // STZ $nnnn
    fn op_9C<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.store(sys, self.base1, MachineInt(0))
    }

    // STA $nnnn,X
    fn op_9D<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, true)?;
        self.store(sys, self.base1, self.a)
    }

    // STZ $nnnn,X
    fn op_9E<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, true)?;
        self.store(sys, self.base1, MachineInt(0))
    }

    // NOP (single-cycle)
    fn op_9F<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // LDY #nn
    fn op_A0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.y = self.immediate(sys)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA ($nn,X)
    fn op_A1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izx(sys)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX #nn
    fn op_A2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.x = self.immediate(sys)?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_A3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // LDY $nn
    fn op_A4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nn
    fn op_A5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nn
    fn op_A6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_A7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // TAY
    fn op_A8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.y = self.a;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA #nn
    fn op_A9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.a = self.immediate(sys)?;
        self.flags.nz(self.a);
        Some(())
    }

    // TAX
    fn op_AA<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.x = self.a;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_AB<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // LDY $nnnn
    fn op_AC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nnnn
    fn op_AD<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nnnn
    fn op_AE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_AF<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BCS
    fn op_B0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, self.flags.c())
    }

    // LDA ($nn),Y
    fn op_B1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izy(sys, false)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDA ($nn)
    fn op_B2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izp(sys)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // NOP (single-cycle)
    fn op_B3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // LDY $nn,X
    fn op_B4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nn,X
    fn op_B5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nn,Y
    fn op_B6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.y)?;
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_B7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CLV
    fn op_B8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.flags.set_v(false);
        Some(())
    }

    // LDA $nnnn,Y
    fn op_B9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // TSX
    fn op_BA<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.x = self.sp;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_BB<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // LDY $nnnn,X
    fn op_BC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nnnn,X
    fn op_BD<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nnnn,Y
    fn op_BE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_BF<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPY #nn
    fn op_C0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        let val = self.immediate(sys)?;
        self.CMP(self.y, val);
        Some(())
    }

    // CMP ($nn,X)
    fn op_C1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izx(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // NOP #nn
    fn op_C2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.immediate(sys)?;
        Some(())
    }

    // NOP (single-cycle)
    fn op_C3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPY $nn
    fn op_C4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.y, val);
        Some(())
    }

    // CMP $nn
    fn op_C5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nn
    fn op_C6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.rmw(sys, self.base1, Cmos::DEC)
    }

    // NOP (single-cycle)
    fn op_C7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // INY
    fn op_C8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.y += 1;
        self.flags.nz(self.y);
        Some(())
    }

    // CMP #nn
    fn op_C9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        let val = self.immediate(sys)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEX
    fn op_CA<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.x -= 1;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_CB<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPY $nnnn
    fn op_CC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.y, val);
        Some(())
    }

    // CMP $nnnn
    fn op_CD<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nnnn
    fn op_CE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::DEC)
    }

    // NOP (single-cycle)
    fn op_CF<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BNE
    fn op_D0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, !self.flags.z())
    }

    // CMP ($nn),Y
    fn op_D1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izy(sys, false)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // CMP ($nn)
    fn op_D2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // NOP (single-cycle)
    fn op_D3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP $nn,X
    fn op_D4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.load(sys, self.base1)?;
        Some(())
    }

    // CMP $nn,X
    fn op_D5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nn,X
    fn op_D6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.rmw(sys, self.base1, Cmos::DEC)
    }

    // NOP (single-cycle)
    fn op_D7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CLD
    fn op_D8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.flags.d = false;
        Some(())
    }

    // CMP $nnnn,Y
    fn op_D9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // PHX
    fn op_DA<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.store(sys, Addr::stack(self.sp), self.x)?;
        self.sp -= 1;
        Some(())
    }

    // NOP (single-cycle)
    fn op_DB<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP $nnnn,X (4-cycle)
    fn op_DC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.load(sys, self.base1.no_carry(self.x))?;
        Some(())
    }

    // CMP $nnnn,X
    fn op_DD<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nnnn,X
    fn op_DE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, true)?;
        self.rmw(sys, self.base1, Cmos::DEC)
    }

    // NOP (single-cycle)
    fn op_DF<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPX #nn
    fn op_E0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        let val = self.immediate(sys)?;
        self.CMP(self.x, val);
        Some(())
    }

    // SBC ($nn,X)
    fn op_E1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izx(sys)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // NOP #nn
    fn op_E2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.immediate(sys)?;
        Some(())
    }

    // NOP (single-cycle)
    fn op_E3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPX $nn
    fn op_E4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.x, val);
        Some(())
    }

    // SBC $nn
    fn op_E5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // INC $nn
    fn op_E6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zp(sys)?;
        self.rmw(sys, self.base1, Cmos::INC)
    }

    // NOP (single-cycle)
    fn op_E7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // INX
    fn op_E8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.x += 1;
        self.flags.nz(self.x);
        Some(())
    }

    // SBC #nn
    fn op_E9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
    fn op_EA<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)
    }

    // NOP (single-cycle)
    fn op_EB<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // CPX $nnnn
    fn op_EC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        let val = self.load(sys, self.base1)?;
        self.CMP(self.x, val);
        Some(())
    }

    // SBC $nnnn
    fn op_ED<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // INC $nnnn
    fn op_EE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.rmw(sys, self.base1, Cmos::INC)
    }

    // NOP (single-cycle)
    fn op_EF<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // BEQ
    fn op_F0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.branch(sys, self.flags.z())
    }

    // SBC ($nn),Y
    fn op_F1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izy(sys, false)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // SBC ($nn)
    fn op_F2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_izp(sys)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // NOP (single-cycle)
    fn op_F3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP $nn,X
    fn op_F4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.load(sys, self.base1)?;
        Some(())
    }

    // SBC $nn,X
    fn op_F5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // INC $nn,X
    fn op_F6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_zpi(sys, self.x)?;
        self.rmw(sys, self.base1, Cmos::INC)
    }

    // NOP (single-cycle)
    fn op_F7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // SED
    fn op_F8<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.implicit(sys)?;
        self.flags.d = true;
        Some(())
    }

    // SBC $nnnn,Y
    fn op_F9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.y, false)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // PLX
    fn op_FA<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.read(sys, self.pc)?;
        self.read_stack(sys)?;
        self.sp += 1;
        self.x = self.load(sys, Addr::stack(self.sp))?;
        self.flags.nz(self.x);
        Some(())
    }

    // NOP (single-cycle)
    fn op_FB<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.poll_prev_signals(sys);
        Some(())
    }

    // NOP $nnnn,X (4-cycle)
    fn op_FC<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abs(sys)?;
        self.load(sys, self.base1.no_carry(self.x))?;
        Some(())
    }

    // SBC $nnnn,X
    fn op_FD<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, false)?;
        self.decimal(sys, self.base1, Cmos::SBC)
    }

    // INC $nnnn,X
    fn op_FE<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.base1 = self.addr_abi(sys, self.x, true)?;
        self.rmw(sys, self.base1, Cmos::INC)
    }

    // NOP (single-cycle)
    fn op_FF<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
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
            0x0a => self.op_0A(sys)?,
            0x0b => self.op_0B(sys)?,
            0x0c => self.op_0C(sys)?,
            0x0d => self.op_0D(sys)?,
            0x0e => self.op_0E(sys)?,
            0x0f => self.op_0F(sys)?,
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
            0x1a => self.op_1A(sys)?,
            0x1b => self.op_1B(sys)?,
            0x1c => self.op_1C(sys)?,
            0x1d => self.op_1D(sys)?,
            0x1e => self.op_1E(sys)?,
            0x1f => self.op_1F(sys)?,
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
            0x2a => self.op_2A(sys)?,
            0x2b => self.op_2B(sys)?,
            0x2c => self.op_2C(sys)?,
            0x2d => self.op_2D(sys)?,
            0x2e => self.op_2E(sys)?,
            0x2f => self.op_2F(sys)?,
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
            0x3a => self.op_3A(sys)?,
            0x3b => self.op_3B(sys)?,
            0x3c => self.op_3C(sys)?,
            0x3d => self.op_3D(sys)?,
            0x3e => self.op_3E(sys)?,
            0x3f => self.op_3F(sys)?,
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
            0x4a => self.op_4A(sys)?,
            0x4b => self.op_4B(sys)?,
            0x4c => self.op_4C(sys)?,
            0x4d => self.op_4D(sys)?,
            0x4e => self.op_4E(sys)?,
            0x4f => self.op_4F(sys)?,
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
            0x5a => self.op_5A(sys)?,
            0x5b => self.op_5B(sys)?,
            0x5c => self.op_5C(sys)?,
            0x5d => self.op_5D(sys)?,
            0x5e => self.op_5E(sys)?,
            0x5f => self.op_5F(sys)?,
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
            0x6a => self.op_6A(sys)?,
            0x6b => self.op_6B(sys)?,
            0x6c => self.op_6C(sys)?,
            0x6d => self.op_6D(sys)?,
            0x6e => self.op_6E(sys)?,
            0x6f => self.op_6F(sys)?,
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
            0x7a => self.op_7A(sys)?,
            0x7b => self.op_7B(sys)?,
            0x7c => self.op_7C(sys)?,
            0x7d => self.op_7D(sys)?,
            0x7e => self.op_7E(sys)?,
            0x7f => self.op_7F(sys)?,
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
            0x8a => self.op_8A(sys)?,
            0x8b => self.op_8B(sys)?,
            0x8c => self.op_8C(sys)?,
            0x8d => self.op_8D(sys)?,
            0x8e => self.op_8E(sys)?,
            0x8f => self.op_8F(sys)?,
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
            0x9a => self.op_9A(sys)?,
            0x9b => self.op_9B(sys)?,
            0x9c => self.op_9C(sys)?,
            0x9d => self.op_9D(sys)?,
            0x9e => self.op_9E(sys)?,
            0x9f => self.op_9F(sys)?,
            0xa0 => self.op_A0(sys)?,
            0xa1 => self.op_A1(sys)?,
            0xa2 => self.op_A2(sys)?,
            0xa3 => self.op_A3(sys)?,
            0xa4 => self.op_A4(sys)?,
            0xa5 => self.op_A5(sys)?,
            0xa6 => self.op_A6(sys)?,
            0xa7 => self.op_A7(sys)?,
            0xa8 => self.op_A8(sys)?,
            0xa9 => self.op_A9(sys)?,
            0xaa => self.op_AA(sys)?,
            0xab => self.op_AB(sys)?,
            0xac => self.op_AC(sys)?,
            0xad => self.op_AD(sys)?,
            0xae => self.op_AE(sys)?,
            0xaf => self.op_AF(sys)?,
            0xb0 => self.op_B0(sys)?,
            0xb1 => self.op_B1(sys)?,
            0xb2 => self.op_B2(sys)?,
            0xb3 => self.op_B3(sys)?,
            0xb4 => self.op_B4(sys)?,
            0xb5 => self.op_B5(sys)?,
            0xb6 => self.op_B6(sys)?,
            0xb7 => self.op_B7(sys)?,
            0xb8 => self.op_B8(sys)?,
            0xb9 => self.op_B9(sys)?,
            0xba => self.op_BA(sys)?,
            0xbb => self.op_BB(sys)?,
            0xbc => self.op_BC(sys)?,
            0xbd => self.op_BD(sys)?,
            0xbe => self.op_BE(sys)?,
            0xbf => self.op_BF(sys)?,
            0xc0 => self.op_C0(sys)?,
            0xc1 => self.op_C1(sys)?,
            0xc2 => self.op_C2(sys)?,
            0xc3 => self.op_C3(sys)?,
            0xc4 => self.op_C4(sys)?,
            0xc5 => self.op_C5(sys)?,
            0xc6 => self.op_C6(sys)?,
            0xc7 => self.op_C7(sys)?,
            0xc8 => self.op_C8(sys)?,
            0xc9 => self.op_C9(sys)?,
            0xca => self.op_CA(sys)?,
            0xcb => self.op_CB(sys)?,
            0xcc => self.op_CC(sys)?,
            0xcd => self.op_CD(sys)?,
            0xce => self.op_CE(sys)?,
            0xcf => self.op_CF(sys)?,
            0xd0 => self.op_D0(sys)?,
            0xd1 => self.op_D1(sys)?,
            0xd2 => self.op_D2(sys)?,
            0xd3 => self.op_D3(sys)?,
            0xd4 => self.op_D4(sys)?,
            0xd5 => self.op_D5(sys)?,
            0xd6 => self.op_D6(sys)?,
            0xd7 => self.op_D7(sys)?,
            0xd8 => self.op_D8(sys)?,
            0xd9 => self.op_D9(sys)?,
            0xda => self.op_DA(sys)?,
            0xdb => self.op_DB(sys)?,
            0xdc => self.op_DC(sys)?,
            0xdd => self.op_DD(sys)?,
            0xde => self.op_DE(sys)?,
            0xdf => self.op_DF(sys)?,
            0xe0 => self.op_E0(sys)?,
            0xe1 => self.op_E1(sys)?,
            0xe2 => self.op_E2(sys)?,
            0xe3 => self.op_E3(sys)?,
            0xe4 => self.op_E4(sys)?,
            0xe5 => self.op_E5(sys)?,
            0xe6 => self.op_E6(sys)?,
            0xe7 => self.op_E7(sys)?,
            0xe8 => self.op_E8(sys)?,
            0xe9 => self.op_E9(sys)?,
            0xea => self.op_EA(sys)?,
            0xeb => self.op_EB(sys)?,
            0xec => self.op_EC(sys)?,
            0xed => self.op_ED(sys)?,
            0xee => self.op_EE(sys)?,
            0xef => self.op_EF(sys)?,
            0xf0 => self.op_F0(sys)?,
            0xf1 => self.op_F1(sys)?,
            0xf2 => self.op_F2(sys)?,
            0xf3 => self.op_F3(sys)?,
            0xf4 => self.op_F4(sys)?,
            0xf5 => self.op_F5(sys)?,
            0xf6 => self.op_F6(sys)?,
            0xf7 => self.op_F7(sys)?,
            0xf8 => self.op_F8(sys)?,
            0xf9 => self.op_F9(sys)?,
            0xfa => self.op_FA(sys)?,
            0xfb => self.op_FB(sys)?,
            0xfc => self.op_FC(sys)?,
            0xfd => self.op_FD(sys)?,
            0xfe => self.op_FE(sys)?,
            0xff => self.op_FF(sys)?,
            _ => unreachable!(),
        }
        Some(())
    }
}