// Copyright 2018 Ed McCardell
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::{NmiLength, Nmos, Sys};

use crate::mi::{Addr, AddrExt, AddrMath};

impl Nmos {
    // BRK
    fn cycle_op_00<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            if self.do_int {
                self.read(sys, self.pc)?;
            } else {
                self.fetch_operand(sys)?;
            }
        }

        if self.op_cycle == 2 {
            if self.reset {
                self.read_stack(sys)?;
            } else {
                self.write_stack(sys, self.pc.hi())?;
            }
            self.sp -= 1;
        }

        if self.op_cycle == 3 {
            if self.reset {
                self.read_stack(sys)?;
            } else {
                self.write_stack(sys, self.pc.lo())?;
            }
            self.sp -= 1;
            self.base1 = self.signal_vector(sys);
        }

        if self.op_cycle == 4 {
            if self.reset {
                self.read_stack(sys)?;
            } else if self.do_int {
                self.write_stack(sys, self.flags.to_byte() & 0b1110_1111)?;
            } else {
                self.write_stack(sys, self.flags.to_byte())?;
            }
            self.sp -= 1;
            if !self.nmi
                && sys.peek_nmi()
                && sys.nmi_length() < NmiLength::Plenty
            {
                sys.poll_nmi();
            }
        }

        if self.op_cycle == 5 {
            self.lo_byte = self.read(sys, self.base1)?;
            if !self.nmi && sys.peek_nmi() && sys.nmi_length() < NmiLength::Two
            {
                sys.poll_nmi();
            }
        }

        // op_cycle == 6
        self.hi_byte = self.read(sys, self.base1 + 1)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        self.flags.i = true;
        self.clear_signals();
        Some(())
    }

    // ORA ($nn,X)
    fn cycle_op_01<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // KIL
    fn cycle_op_02<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // SLO ($nn,X)
    fn cycle_op_03<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::ASL, 5)?;
        self.ORA(self.lo_byte);
        Some(())
    }

    // *NOP $nn
    fn cycle_op_04<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        self.load(sys, self.base1)?;
        Some(())
    }

    // ORA $nn
    fn cycle_op_05<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nn
    fn cycle_op_06<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::ASL, 2)
    }

    // SLO $nn
    fn cycle_op_07<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::ASL, 2)?;
        self.ORA(self.lo_byte);
        Some(())
    }

    // PHP
    fn cycle_op_08<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.read(sys, self.pc)?;
        }

        // op_cycle == 2
        self.store(sys, Addr::stack(self.sp), self.flags.to_byte())?;
        self.sp -= 1;
        Some(())
    }

    // cycle_op_09 = op_09
    // cycle_op_0a = op_0a
    // cycle_op_0b = op_0b

    // NOP* $nnnn
    fn cycle_op_0c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        self.load(sys, self.base1)?;
        Some(())
    }

    // ORA $nnnn
    fn cycle_op_0d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nnnn
    fn cycle_op_0e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ASL, 3)
    }

    // SLO $nnnn
    fn cycle_op_0f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ASL, 3)?;
        self.ORA(self.lo_byte);
        Some(())
    }

    // BPL
    fn cycle_op_10<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_branch(sys, !self.flags.n())
    }

    // ORA ($nn),Y
    fn cycle_op_11<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, false)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // KIL
    fn cycle_op_12<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // SLO ($nn),Y
    fn cycle_op_13<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, true)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::ASL, 5)?;
        self.ORA(self.lo_byte);
        Some(())
    }

    // NOP* $nn,X
    fn cycle_op_14<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        self.load(sys, self.base1)?;
        Some(())
    }

    // ORA $nn,X
    fn cycle_op_15<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nn,X
    fn cycle_op_16<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ASL, 3)
    }

    // SLO $nn,X
    fn cycle_op_17<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ASL, 3)?;
        self.ORA(self.lo_byte);
        Some(())
    }

    // cycle_op_18 = op_18

    // ORA $nnnn,Y
    fn cycle_op_19<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // cycle_op_1a = op_1a

    // SLO $nnnn,Y
    fn cycle_op_1b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::ASL, 4)?;
        self.ORA(self.lo_byte);
        Some(())
    }

    // NOP* $nnnn,X
    fn cycle_op_1c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        self.load(sys, self.base1)?;
        Some(())
    }

    // ORA $nnnn,X
    fn cycle_op_1d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.ORA(val);
        Some(())
    }

    // ASL $nnnn,X
    fn cycle_op_1e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::ASL, 4)
    }

    // SLO $nnnn,X
    fn cycle_op_1f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::ASL, 4)?;
        self.ORA(self.lo_byte);
        Some(())
    }

    // JSR $nnnn
    fn cycle_op_20<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.lo_byte = self.fetch_operand(sys)?;
        }

        if self.op_cycle == 2 {
            self.read_stack(sys)?;
        }

        if self.op_cycle == 3 {
            self.write_stack(sys, self.pc.hi())?;
            self.sp -= 1;
        }

        if self.op_cycle == 4 {
            self.write_stack(sys, self.pc.lo())?;
            self.sp -= 1;
            self.poll_signals(sys);
        }

        // op_cycle == 5
        self.hi_byte = self.fetch_operand(sys)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // AND ($nn,X)
    fn cycle_op_21<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // KIL
    fn cycle_op_22<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // RLA ($nn,X)
    fn cycle_op_23<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::ROL, 5)?;
        self.AND(self.lo_byte);
        Some(())
    }

    // BIT $nn
    fn cycle_op_24<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nn
    fn cycle_op_25<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nn
    fn cycle_op_26<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::ROL, 2)
    }

    // RLA $nn
    fn cycle_op_27<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::ROL, 2)?;
        self.AND(self.lo_byte);
        Some(())
    }

    // PLP
    fn cycle_op_28<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.read(sys, self.pc)?;
        }

        if self.op_cycle == 2 {
            self.read_stack(sys)?;
            self.sp += 1;
        }

        // op_cycle == 3
        let p = self.load(sys, Addr::stack(self.sp))?;
        self.flags.from_byte(p);
        Some(())
    }

    // cycle_op_29 = op_29
    // cycle_op_2a = op_2a
    // cycle_op_2b = op_2b

    // BIT $nnnn
    fn cycle_op_2c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.BIT(val);
        Some(())
    }

    // AND $nnnn
    fn cycle_op_2d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nnnn
    fn cycle_op_2e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ROL, 3)
    }

    // RLA $nnnn
    fn cycle_op_2f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ROL, 3)?;
        self.AND(self.lo_byte);
        Some(())
    }

    // BMI
    fn cycle_op_30<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_branch(sys, self.flags.n())
    }

    // AND ($nn),Y
    fn cycle_op_31<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, false)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // KIL
    fn cycle_op_32<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // RLA ($nn),Y
    fn cycle_op_33<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, true)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::ROL, 5)?;
        self.AND(self.lo_byte);
        Some(())
    }

    // NOP* $nn,X
    fn cycle_op_34<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        self.load(sys, self.base1)?;
        Some(())
    }

    // AND $nn,X
    fn cycle_op_35<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nn,X
    fn cycle_op_36<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ROL, 3)
    }

    // RLA $nn,X
    fn cycle_op_37<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ROL, 3)?;
        self.AND(self.lo_byte);
        Some(())
    }

    // cycle_op_38 = op_38

    // AND $nnnn,Y
    fn cycle_op_39<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // cycle_op_3a = op_3a

    // RLA $nnnn,Y
    fn cycle_op_3b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::ROL, 4)?;
        self.AND(self.lo_byte);
        Some(())
    }

    // NOP* $nnnn,X
    fn cycle_op_3c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        self.load(sys, self.base1)?;
        Some(())
    }

    // AND $nnnn,X
    fn cycle_op_3d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.AND(val);
        Some(())
    }

    // ROL $nnnn,X
    fn cycle_op_3e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::ROL, 4)
    }

    // RLA $nnnn,X
    fn cycle_op_3f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::ROL, 4)?;
        self.AND(self.lo_byte);
        Some(())
    }

    // RTI
    fn cycle_op_40<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.read(sys, self.pc)?;
        }

        if self.op_cycle == 2 {
            self.read_stack(sys)?;
            self.sp += 1;
        }

        if self.op_cycle == 3 {
            let p = self.read_stack(sys)?;
            self.sp += 1;
            self.flags.from_byte(p);
        }

        if self.op_cycle == 4 {
            self.lo_byte = self.read_stack(sys)?;
            self.sp += 1;
            self.poll_signals(sys);
        }

        // op_cycle == 5
        self.hi_byte = self.read_stack(sys)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // EOR ($nn,X)
    fn cycle_op_41<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // KIL
    fn cycle_op_42<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // SRE ($nn,X)
    fn cycle_op_43<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::LSR, 5)?;
        self.EOR(self.lo_byte);
        Some(())
    }

    // NOP* $nn
    fn cycle_op_44<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        self.load(sys, self.base1)?;
        Some(())
    }

    // EOR $nn
    fn cycle_op_45<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nn
    fn cycle_op_46<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::LSR, 2)
    }

    // SRE $nn
    fn cycle_op_47<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::LSR, 2)?;
        self.EOR(self.lo_byte);
        Some(())
    }

    // PHA
    fn cycle_op_48<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.read(sys, self.pc)?;
        }

        // op_cycle == 2
        self.store(sys, Addr::stack(self.sp), self.a)?;
        self.sp -= 1;
        Some(())
    }

    // cycle_op_49 = op_49
    // cycle_op_4a = op_4a
    // cycle_op_4b = op_4b

    // JMP $nnnn
    fn cycle_op_4c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.lo_byte = self.fetch_operand(sys)?;
            self.poll_signals(sys);
        }

        // op_cycle == 2
        self.hi_byte = self.fetch_operand(sys)?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // EOR $nnnn
    fn cycle_op_4d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nnnn
    fn cycle_op_4e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::LSR, 3)
    }

    // SRE $nnnn
    fn cycle_op_4f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::LSR, 3)?;
        self.EOR(self.lo_byte);
        Some(())
    }

    // BVC
    fn cycle_op_50<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_branch(sys, !self.flags.v())
    }

    // EOR ($nn),Y
    fn cycle_op_51<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, false)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // KIL
    fn cycle_op_52<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // SRE ($nn,Y)
    fn cycle_op_53<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, true)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::LSR, 5)?;
        self.EOR(self.lo_byte);
        Some(())
    }

    // NOP* $nn,X
    fn cycle_op_54<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        self.load(sys, self.base1)?;
        Some(())
    }

    // EOR $nn,X
    fn cycle_op_55<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nn,X
    fn cycle_op_56<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::LSR, 3)
    }

    // SRE $nn,X
    fn cycle_op_57<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::LSR, 3)?;
        self.EOR(self.lo_byte);
        Some(())
    }

    // cycle_op_58 = op_58

    // EOR $nnnn,Y
    fn cycle_op_59<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // cycle_op_5a = op_5a

    // SRE $nnnn,Y
    fn cycle_op_5b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::LSR, 4)?;
        self.EOR(self.lo_byte);
        Some(())
    }

    // NOP* $nnnn,X
    fn cycle_op_5c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        self.load(sys, self.base1)?;
        Some(())
    }

    // EOR $nnnn,X
    fn cycle_op_5d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.EOR(val);
        Some(())
    }

    // LSR $nnnn,X
    fn cycle_op_5e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::LSR, 4)
    }

    // SRE $nnnn,X
    fn cycle_op_5f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::LSR, 4)?;
        self.EOR(self.lo_byte);
        Some(())
    }

    // RTS
    fn cycle_op_60<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.read(sys, self.pc)?;
        }

        if self.op_cycle == 2 {
            self.read_stack(sys)?;
            self.sp += 1;
        }

        if self.op_cycle == 3 {
            self.lo_byte = self.read_stack(sys)?;
            self.sp += 1;
        }

        if self.op_cycle == 4 {
            self.hi_byte = self.read_stack(sys)?;
            self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
            self.poll_signals(sys);
        }

        // op_cycle == 5
        self.fetch_operand(sys)?;
        Some(())
    }

    // ADC ($nn,X)
    fn cycle_op_61<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.ADC(val);
        Some(())
    }

    // KIL
    fn cycle_op_62<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // RRA ($nn,X)
    fn cycle_op_63<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::ROR, 5)?;
        self.ADC(self.lo_byte);
        Some(())
    }

    // NOP* $nn
    fn cycle_op_64<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        self.load(sys, self.base1)?;
        Some(())
    }

    // ADC $nn
    fn cycle_op_65<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        let val = self.load(sys, self.base1)?;
        self.ADC(val);
        Some(())
    }

    // ROR $nn
    fn cycle_op_66<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::ROR, 2)
    }

    // RRA $nn
    fn cycle_op_67<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::ROR, 2)?;
        self.ADC(self.lo_byte);
        Some(())
    }

    // PLA
    fn cycle_op_68<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.read(sys, self.pc)?;
        }

        if self.op_cycle == 2 {
            self.read_stack(sys)?;
            self.sp += 1;
        }

        // op_cycle == 3
        self.a = self.load(sys, Addr::stack(self.sp))?;
        self.flags.nz(self.a);
        Some(())
    }

    // cycle_op_69 = op_69
    // cycle_op_6a = op_6a
    // cycle_op_6b = op_6b

    // JMP ($nnnn)
    fn cycle_op_6c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        if self.op_cycle == 3 {
            self.lo_byte = self.read(sys, self.base1)?;
        }

        // op_cycle == 4
        self.hi_byte = self.load(sys, self.base1.no_carry(1))?;
        self.pc = Addr::from_bytes(self.lo_byte, self.hi_byte);
        Some(())
    }

    // ADC $nnnn
    fn cycle_op_6d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.ADC(val);
        Some(())
    }

    // ROR $nnnn
    fn cycle_op_6e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ROR, 3)
    }

    fn cycle_op_6f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ROR, 3)?;
        self.ADC(self.lo_byte);
        Some(())
    }

    // BVS
    fn cycle_op_70<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_branch(sys, self.flags.v())
    }

    // ADC ($nn),Y
    fn cycle_op_71<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, false)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.ADC(val);
        Some(())
    }

    // KIL
    fn cycle_op_72<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // RRA ($nn),Y
    fn cycle_op_73<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, true)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::ROR, 5)?;
        self.ADC(self.lo_byte);
        Some(())
    }

    // NOP* $nn,X
    fn cycle_op_74<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        self.load(sys, self.base1)?;
        Some(())
    }

    // ADC $nn,X
    fn cycle_op_75<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.ADC(val);
        Some(())
    }

    // ROR $nn,X
    fn cycle_op_76<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ROR, 3)
    }

    // RRA $nn,X
    fn cycle_op_77<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::ROR, 3)?;
        self.ADC(self.lo_byte);
        Some(())
    }

    // cycle_op_78 = op_78

    // ADC $nnnn,Y
    fn cycle_op_79<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.ADC(val);
        Some(())
    }

    // cycle_op_7a = op_7a

    // RRA $nnnn,Y
    fn cycle_op_7b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::ROR, 4)?;
        self.ADC(self.lo_byte);
        Some(())
    }

    // NOP* $nnnn,X
    fn cycle_op_7c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        self.load(sys, self.base1)?;
        Some(())
    }

    // ADC $nnnn,X
    fn cycle_op_7d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.ADC(val);
        Some(())
    }

    // ROR $nnnn,X
    fn cycle_op_7e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::ROR, 4)
    }

    // RRA $nnnn,X
    fn cycle_op_7f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::ROR, 4)?;
        self.ADC(self.lo_byte);
        Some(())
    }

    // cycle_op_80 = op_80

    // STA ($nn,X)
    fn cycle_op_81<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle == 5
        self.store(sys, self.base1, self.a)
    }

    // cycle_op_82 = op_82

    // SAX ($nn,X)
    fn cycle_op_83<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle == 5
        self.store(sys, self.base1, self.a & self.x)
    }

    // STY $nn
    fn cycle_op_84<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        self.store(sys, self.base1, self.y)
    }

    // STA $nn
    fn cycle_op_85<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        self.store(sys, self.base1, self.a)
    }

    // STX $nn
    fn cycle_op_86<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        self.store(sys, self.base1, self.x)
    }

    // SAX $nn
    fn cycle_op_87<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        self.store(sys, self.base1, self.a & self.x)
    }

    // cycle_op_88 = op_88
    // cycle_op_89 = op_89
    // cycle_op_8a = op_8a
    // cycle_op_8b = op_8b

    // STY $nnnn
    fn cycle_op_8c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        self.store(sys, self.base1, self.y)
    }

    // STA $nnnn
    fn cycle_op_8d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        self.store(sys, self.base1, self.a)
    }

    // STX $nnnn
    fn cycle_op_8e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        self.store(sys, self.base1, self.x)
    }

    // SAX $nnnn
    fn cycle_op_8f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        self.store(sys, self.base1, self.a & self.x)
    }

    // BCC
    fn cycle_op_90<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_branch(sys, !self.flags.c())
    }

    // STA ($nn),Y
    fn cycle_op_91<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, true)?;
        }

        // op_cycle == 5
        self.store(sys, self.base1, self.a)
    }

    // KIL
    fn cycle_op_92<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // AHX ($nn),Y
    fn cycle_op_93<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        if self.op_cycle < 4 {
            self.base1 = self.cycle_fetch_vector_zp(sys, self.base1, 2)?;
        }

        if self.op_cycle == 4 {
            // TODO: match and check if sys.rdy to remove &{H+1}
            self.read(sys, self.base1.no_carry(self.y))?;
            self.lo_byte = self.a & self.x & (self.base1.hi() + 1);
            if self.base1.check_carry(self.y) {
                self.base1 =
                    Addr::from_bytes((self.base1 + self.y).lo(), self.lo_byte);
            } else {
                self.base1 += self.y
            }
        }

        // op_cycle == 5
        self.store(sys, self.base1, self.lo_byte)
    }

    // STY $nn,X
    fn cycle_op_94<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        self.store(sys, self.base1, self.y)
    }

    // STA $nn,X
    fn cycle_op_95<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        self.store(sys, self.base1, self.a)
    }

    // STX $nn,Y
    fn cycle_op_96<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.y)?;
        }

        // op_cycle == 3
        self.store(sys, self.base1, self.x)
    }

    // SAX $nn,Y
    fn cycle_op_97<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.y)?;
        }

        // op_cycle == 3
        self.store(sys, self.base1, self.a & self.x)
    }

    // cycle_op_98 = op_98

    // STA $nnnn,Y
    fn cycle_op_99<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, true)?;
        }

        // op_cycle == 4
        self.store(sys, self.base1, self.a)
    }

    // cycle_op_9a = op_9a

    // TAS $nnnn,Y
    fn cycle_op_9b<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        if self.op_cycle == 3 {
            // TODO: match and check if sys.rdy to remove &{H+1}
            self.read(sys, self.base1.no_carry(self.y))?;
            self.sp = self.a & self.x;
            self.lo_byte = self.a & self.x & (self.base1.hi() + 1);
            if self.base1.check_carry(self.y) {
                self.base1 =
                    Addr::from_bytes((self.base1 + self.y).lo(), self.lo_byte);
            } else {
                self.base1 += self.y
            }
        }

        // op_cycle == 4
        self.store(sys, self.base1, self.lo_byte)
    }

    // SHY $nnnn,X
    fn cycle_op_9c<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        if self.op_cycle == 3 {
            // TODO: match and check if sys.rdy to remove &{H+1}
            self.read(sys, self.base1.no_carry(self.x))?;
            self.lo_byte = self.y & (self.base1.hi() + 1);
            if self.base1.check_carry(self.x) {
                self.base1 =
                    Addr::from_bytes((self.base1 + self.x).lo(), self.lo_byte);
            } else {
                self.base1 += self.x
            }
        }

        // op_cycle == 4
        self.store(sys, self.base1, self.lo_byte)
    }

    // STA $nnnn,X
    fn cycle_op_9d<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle == 4
        self.store(sys, self.base1, self.a)
    }

    // SHX $nnnn,Y
    fn cycle_op_9e<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        if self.op_cycle == 3 {
            // TODO: match and check if sys.rdy to remove &{H+1}
            self.read(sys, self.base1.no_carry(self.y))?;
            self.lo_byte = self.x & (self.base1.hi() + 1);
            if self.base1.check_carry(self.y) {
                self.base1 =
                    Addr::from_bytes((self.base1 + self.y).lo(), self.lo_byte);
            } else {
                self.base1 += self.y
            }
        }

        // op_cycle == 4
        self.store(sys, self.base1, self.lo_byte)
    }

    // AHX $nnnn,Y
    fn cycle_op_9f<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        if self.op_cycle == 3 {
            // TODO: match and check if sys.rdy to remove &{H+1}
            self.read(sys, self.base1.no_carry(self.y))?;
            self.lo_byte = self.a & self.x & (self.base1.hi() + 1);
            if self.base1.check_carry(self.y) {
                self.base1 =
                    Addr::from_bytes((self.base1 + self.y).lo(), self.lo_byte);
            } else {
                self.base1 += self.y
            }
        }

        // op_cycle == 4
        self.store(sys, self.base1, self.lo_byte)
    }

    // cycle_op_a0 = op_a0

    // LDA ($nn,X)
    fn cycle_op_a1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle == 5
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // cycle_op_a2 = op_a2

    // LAX ($nn,X)
    fn cycle_op_a3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle == 5
        self.x = self.load(sys, self.base1)?;
        self.a = self.x;
        self.flags.nz(self.x);
        Some(())
    }

    // LDY $nn
    fn cycle_op_a4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nn
    fn cycle_op_a5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nn
    fn cycle_op_a6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // LAX $nn
    fn cycle_op_a7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        self.x = self.load(sys, self.base1)?;
        self.a = self.x;
        self.flags.nz(self.x);
        Some(())
    }

    // cycle_op_a8 = op_a8
    // cycle_op_a9 = op_a9
    // cycle_op_aa = op_aa
    // cycle_op_ab = op_ab

    // LDY $nnnn
    fn cycle_op_ac<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nnnn
    fn cycle_op_ad<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nnnn
    fn cycle_op_ae<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // LAX $nnnn
    fn cycle_op_af<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        self.x = self.load(sys, self.base1)?;
        self.a = self.x;
        self.flags.nz(self.x);
        Some(())
    }

    // BCS
    fn cycle_op_b0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_branch(sys, self.flags.c())
    }

    // LDA ($nn),Y
    fn cycle_op_b1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, false)?;
        }

        // op_cycle == 5
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // KIL
    fn cycle_op_b2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // LAX ($nn,Y)
    fn cycle_op_b3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, false)?;
        }

        // op_cycle == 5
        self.x = self.load(sys, self.base1)?;
        self.a = self.x;
        self.flags.nz(self.x);
        Some(())
    }

    // LDY $nn,X
    fn cycle_op_b4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nn,X
    fn cycle_op_b5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nn,Y
    fn cycle_op_b6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.y)?;
        }

        // op_cycle == 3
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // LAX $nn,Y
    fn cycle_op_b7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.y)?;
        }

        // op_cycle == 3
        self.x = self.load(sys, self.base1)?;
        self.a = self.x;
        self.flags.nz(self.x);
        Some(())
    }

    // cycle_op_b8 = op_b8

    // LDA $nnnn,Y
    fn cycle_op_b9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, false)?;
        }

        // op_cycle == 4
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // cycle_op_ba = op_ba

    // LAS $nnnn,Y
    fn cycle_op_bb<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.sp &= val;
        self.a = self.sp;
        self.x = self.sp;
        Some(())
    }

    // LDY $nnnn,X
    fn cycle_op_bc<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        self.y = self.load(sys, self.base1)?;
        self.flags.nz(self.y);
        Some(())
    }

    // LDA $nnnn,X
    fn cycle_op_bd<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        self.a = self.load(sys, self.base1)?;
        self.flags.nz(self.a);
        Some(())
    }

    // LDX $nnnn,Y
    fn cycle_op_be<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, false)?;
        }

        // op_cycle == 4
        self.x = self.load(sys, self.base1)?;
        self.flags.nz(self.x);
        Some(())
    }

    // LAX $nnnn,Y
    fn cycle_op_bf<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, false)?;
        }

        // op_cycle == 4
        self.x = self.load(sys, self.base1)?;
        self.a = self.x;
        self.flags.nz(self.x);
        Some(())
    }

    // cycle_op_c0 = op_c0

    // CMP ($nn,X)
    fn cycle_op_c1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // cycle_op_c2 = op_c2

    // DCP ($nn,X)
    fn cycle_op_c3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::DEC, 5)?;
        self.CMP(self.a, self.lo_byte);
        Some(())
    }

    // CPY $nn
    fn cycle_op_c4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        let val = self.load(sys, self.base1)?;
        self.CMP(self.y, val);
        Some(())
    }

    // CMP $nn
    fn cycle_op_c5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nn
    fn cycle_op_c6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::DEC, 2)
    }

    // DCP $nn
    fn cycle_op_c7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::DEC, 2)?;
        self.CMP(self.a, self.lo_byte);
        Some(())
    }

    // cycle_op_c8 = op_c8
    // cycle_op_c9 = op_c9
    // cycle_op_ca = op_ca
    // cycle_op_cb = op_cb

    // CPY $nnnn
    fn cycle_op_cc<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.CMP(self.y, val);
        Some(())
    }

    // CMP $nnnn
    fn cycle_op_cd<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nnnn
    fn cycle_op_ce<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::DEC, 3)
    }

    // DCP $nnnn
    fn cycle_op_cf<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::DEC, 3)?;
        self.CMP(self.a, self.lo_byte);
        Some(())
    }

    // BNE
    fn cycle_op_d0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_branch(sys, !self.flags.z())
    }

    // CMP($nn),Y
    fn cycle_op_d1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, false)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // KIL
    fn cycle_op_d2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // DCP ($nn),Y
    fn cycle_op_d3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, true)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::DEC, 5)?;
        self.CMP(self.a, self.lo_byte);
        Some(())
    }

    // NOP* $nn,X
    fn cycle_op_d4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        self.load(sys, self.base1)?;
        Some(())
    }

    // CMP $nn,X
    fn cycle_op_d5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nn,X
    fn cycle_op_d6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::DEC, 3)
    }

    // DCP $nn,X
    fn cycle_op_d7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::DEC, 3)?;
        self.CMP(self.a, self.lo_byte);
        Some(())
    }

    // cycle_op_d8 = op_d8

    // CMP $nnnn,Y
    fn cycle_op_d9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // cycle_op_da = op_da

    // DCP $nnnn,Y
    fn cycle_op_db<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::DEC, 4)?;
        self.CMP(self.a, self.lo_byte);
        Some(())
    }

    // NOP* $nnnn,X
    fn cycle_op_dc<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        self.load(sys, self.base1)?;
        Some(())
    }

    // CMP $nnnn,X
    fn cycle_op_dd<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.CMP(self.a, val);
        Some(())
    }

    // DEC $nnnn,X
    fn cycle_op_de<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::DEC, 4)
    }

    // DCP $nnnn,X
    fn cycle_op_df<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::DEC, 4)?;
        self.CMP(self.a, self.lo_byte);
        Some(())
    }

    // cycle_op_e0 = op_e0

    // SBC ($nn,X)
    fn cycle_op_e1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.SBC(val);
        Some(())
    }

    // cycle_op_e2 = op_e2

    // ISC ($nn,X)
    fn cycle_op_e3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izx(sys)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::INC, 5)?;
        self.SBC(self.lo_byte);
        Some(())
    }

    // CPX $nn
    fn cycle_op_e4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        let val = self.load(sys, self.base1)?;
        self.CMP(self.x, val);
        Some(())
    }

    // SBC $nn
    fn cycle_op_e5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle == 2
        let val = self.load(sys, self.base1)?;
        self.SBC(val);
        Some(())
    }

    // INC $nn
    fn cycle_op_e6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::INC, 2)
    }

    // ISC $nn
    fn cycle_op_e7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle == 1 {
            self.base1 = self.addr_zp(sys)?;
        }

        // op_cycle >= 2
        self.cycle_rmw(sys, self.base1, Nmos::INC, 2)?;
        self.SBC(self.lo_byte);
        Some(())
    }

    // cycle_op_e8 = op_e8
    // cycle_op_e9 = op_e9
    // cycle_op_ea = op_ea
    // cycle_op_eb = op_eb

    // CPX $nnnn
    fn cycle_op_ec<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.CMP(self.x, val);
        Some(())
    }

    // SBC $nnnn
    fn cycle_op_ed<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.SBC(val);
        Some(())
    }

    // INC $nnnn
    fn cycle_op_ee<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::INC, 3)
    }

    // ISC $nnnn
    fn cycle_op_ef<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_abs(sys)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::INC, 3)?;
        self.SBC(self.lo_byte);
        Some(())
    }

    // BEQ
    fn cycle_op_f0<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_branch(sys, self.flags.z())
    }

    // SBC ($nn),Y
    fn cycle_op_f1<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, false)?;
        }

        // op_cycle == 5
        let val = self.load(sys, self.base1)?;
        self.SBC(val);
        Some(())
    }

    // KIL
    fn cycle_op_f2<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        self.cycle_halt(sys)
    }

    // ISC ($nn),Y
    fn cycle_op_f3<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 5 {
            self.base1 = self.cycle_addr_izy(sys, true)?;
        }

        // op_cycle >= 5
        self.cycle_rmw(sys, self.base1, Nmos::INC, 5)?;
        self.SBC(self.lo_byte);
        Some(())
    }

    // NOP* $nn,X
    fn cycle_op_f4<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        self.load(sys, self.base1)?;
        Some(())
    }

    // SBC $nn,X
    fn cycle_op_f5<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle == 3
        let val = self.load(sys, self.base1)?;
        self.SBC(val);
        Some(())
    }

    // INC $nn,X
    fn cycle_op_f6<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::INC, 3)
    }

    // ISC $nn,X
    fn cycle_op_f7<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 3 {
            self.base1 = self.cycle_addr_zpi(sys, self.x)?;
        }

        // op_cycle >= 3
        self.cycle_rmw(sys, self.base1, Nmos::INC, 3)?;
        self.SBC(self.lo_byte);
        Some(())
    }

    // cycle_op_f8 = op_f8

    // SBC $nnnn,Y
    fn cycle_op_f9<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.SBC(val);
        Some(())
    }

    // cycle_op_fa = op_fa

    // ISC $nnnn,Y
    fn cycle_op_fb<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.y, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::INC, 4)?;
        self.SBC(self.lo_byte);
        Some(())
    }

    // NOP* $nnnn,X
    fn cycle_op_fc<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        self.load(sys, self.base1)?;
        Some(())
    }

    // SBC $nnnn,X
    fn cycle_op_fd<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, false)?;
        }

        // op_cycle == 4
        let val = self.load(sys, self.base1)?;
        self.SBC(val);
        Some(())
    }

    // INC $nnnn,X
    fn cycle_op_fe<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::INC, 4)
    }

    // ISC $nnnn,X
    fn cycle_op_ff<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        if self.op_cycle < 4 {
            self.base1 = self.cycle_addr_abi(sys, self.x, true)?;
        }

        // op_cycle >= 4
        self.cycle_rmw(sys, self.base1, Nmos::INC, 4)?;
        self.SBC(self.lo_byte);
        Some(())
    }
}

impl Nmos {
    #[cfg_attr(
        feature = "cargo-clippy",
        allow(clippy::cyclomatic_complexity)
    )]
    pub(crate) fn cycle_exec<S: Sys>(&mut self, sys: &mut S) -> Option<()> {
        match self.op {
            0x00 => self.cycle_op_00(sys)?,
            0x01 => self.cycle_op_01(sys)?,
            0x02 => self.cycle_op_02(sys)?,
            0x03 => self.cycle_op_03(sys)?,
            0x04 => self.cycle_op_04(sys)?,
            0x05 => self.cycle_op_05(sys)?,
            0x06 => self.cycle_op_06(sys)?,
            0x07 => self.cycle_op_07(sys)?,
            0x08 => self.cycle_op_08(sys)?,
            0x09 => self.op_09(sys)?,
            0x0a => self.op_0a(sys)?,
            0x0b => self.op_0b(sys)?,
            0x0c => self.cycle_op_0c(sys)?,
            0x0d => self.cycle_op_0d(sys)?,
            0x0e => self.cycle_op_0e(sys)?,
            0x0f => self.cycle_op_0f(sys)?,
            0x10 => self.cycle_op_10(sys)?,
            0x11 => self.cycle_op_11(sys)?,
            0x12 => self.cycle_op_12(sys)?,
            0x13 => self.cycle_op_13(sys)?,
            0x14 => self.cycle_op_14(sys)?,
            0x15 => self.cycle_op_15(sys)?,
            0x16 => self.cycle_op_16(sys)?,
            0x17 => self.cycle_op_17(sys)?,
            0x18 => self.op_18(sys)?,
            0x19 => self.cycle_op_19(sys)?,
            0x1a => self.op_1a(sys)?,
            0x1b => self.cycle_op_1b(sys)?,
            0x1c => self.cycle_op_1c(sys)?,
            0x1d => self.cycle_op_1d(sys)?,
            0x1e => self.cycle_op_1e(sys)?,
            0x1f => self.cycle_op_1f(sys)?,
            0x20 => self.cycle_op_20(sys)?,
            0x21 => self.cycle_op_21(sys)?,
            0x22 => self.cycle_op_22(sys)?,
            0x23 => self.cycle_op_23(sys)?,
            0x24 => self.cycle_op_24(sys)?,
            0x25 => self.cycle_op_25(sys)?,
            0x26 => self.cycle_op_26(sys)?,
            0x27 => self.cycle_op_27(sys)?,
            0x28 => self.cycle_op_28(sys)?,
            0x29 => self.op_29(sys)?,
            0x2a => self.op_2a(sys)?,
            0x2b => self.op_2b(sys)?,
            0x2c => self.cycle_op_2c(sys)?,
            0x2d => self.cycle_op_2d(sys)?,
            0x2e => self.cycle_op_2e(sys)?,
            0x2f => self.cycle_op_2f(sys)?,
            0x30 => self.cycle_op_30(sys)?,
            0x31 => self.cycle_op_31(sys)?,
            0x32 => self.cycle_op_32(sys)?,
            0x33 => self.cycle_op_33(sys)?,
            0x34 => self.cycle_op_34(sys)?,
            0x35 => self.cycle_op_35(sys)?,
            0x36 => self.cycle_op_36(sys)?,
            0x37 => self.cycle_op_37(sys)?,
            0x38 => self.op_38(sys)?,
            0x39 => self.cycle_op_39(sys)?,
            0x3a => self.op_3a(sys)?,
            0x3b => self.cycle_op_3b(sys)?,
            0x3c => self.cycle_op_3c(sys)?,
            0x3d => self.cycle_op_3d(sys)?,
            0x3e => self.cycle_op_3e(sys)?,
            0x3f => self.cycle_op_3f(sys)?,
            0x40 => self.cycle_op_40(sys)?,
            0x41 => self.cycle_op_41(sys)?,
            0x42 => self.cycle_op_42(sys)?,
            0x43 => self.cycle_op_43(sys)?,
            0x44 => self.cycle_op_44(sys)?,
            0x45 => self.cycle_op_45(sys)?,
            0x46 => self.cycle_op_46(sys)?,
            0x47 => self.cycle_op_47(sys)?,
            0x48 => self.cycle_op_48(sys)?,
            0x49 => self.op_49(sys)?,
            0x4a => self.op_4a(sys)?,
            0x4b => self.op_4b(sys)?,
            0x4c => self.cycle_op_4c(sys)?,
            0x4d => self.cycle_op_4d(sys)?,
            0x4e => self.cycle_op_4e(sys)?,
            0x4f => self.cycle_op_4f(sys)?,
            0x50 => self.cycle_op_50(sys)?,
            0x51 => self.cycle_op_51(sys)?,
            0x52 => self.cycle_op_52(sys)?,
            0x53 => self.cycle_op_53(sys)?,
            0x54 => self.cycle_op_54(sys)?,
            0x55 => self.cycle_op_55(sys)?,
            0x56 => self.cycle_op_56(sys)?,
            0x57 => self.cycle_op_57(sys)?,
            0x58 => self.op_58(sys)?,
            0x59 => self.cycle_op_59(sys)?,
            0x5a => self.op_5a(sys)?,
            0x5b => self.cycle_op_5b(sys)?,
            0x5c => self.cycle_op_5c(sys)?,
            0x5d => self.cycle_op_5d(sys)?,
            0x5e => self.cycle_op_5e(sys)?,
            0x5f => self.cycle_op_5f(sys)?,
            0x60 => self.cycle_op_60(sys)?,
            0x61 => self.cycle_op_61(sys)?,
            0x62 => self.cycle_op_62(sys)?,
            0x63 => self.cycle_op_63(sys)?,
            0x64 => self.cycle_op_64(sys)?,
            0x65 => self.cycle_op_65(sys)?,
            0x66 => self.cycle_op_66(sys)?,
            0x67 => self.cycle_op_67(sys)?,
            0x68 => self.cycle_op_68(sys)?,
            0x69 => self.op_69(sys)?,
            0x6a => self.op_6a(sys)?,
            0x6b => self.op_6b(sys)?,
            0x6c => self.cycle_op_6c(sys)?,
            0x6d => self.cycle_op_6d(sys)?,
            0x6e => self.cycle_op_6e(sys)?,
            0x6f => self.cycle_op_6f(sys)?,
            0x70 => self.cycle_op_70(sys)?,
            0x71 => self.cycle_op_71(sys)?,
            0x72 => self.cycle_op_72(sys)?,
            0x73 => self.cycle_op_73(sys)?,
            0x74 => self.cycle_op_74(sys)?,
            0x75 => self.cycle_op_75(sys)?,
            0x76 => self.cycle_op_76(sys)?,
            0x77 => self.cycle_op_77(sys)?,
            0x78 => self.op_78(sys)?,
            0x79 => self.cycle_op_79(sys)?,
            0x7a => self.op_7a(sys)?,
            0x7b => self.cycle_op_7b(sys)?,
            0x7c => self.cycle_op_7c(sys)?,
            0x7d => self.cycle_op_7d(sys)?,
            0x7e => self.cycle_op_7e(sys)?,
            0x7f => self.cycle_op_7f(sys)?,
            0x80 => self.op_80(sys)?,
            0x81 => self.cycle_op_81(sys)?,
            0x82 => self.op_82(sys)?,
            0x83 => self.cycle_op_83(sys)?,
            0x84 => self.cycle_op_84(sys)?,
            0x85 => self.cycle_op_85(sys)?,
            0x86 => self.cycle_op_86(sys)?,
            0x87 => self.cycle_op_87(sys)?,
            0x88 => self.op_88(sys)?,
            0x89 => self.op_89(sys)?,
            0x8a => self.op_8a(sys)?,
            0x8b => self.op_8b(sys)?,
            0x8c => self.cycle_op_8c(sys)?,
            0x8d => self.cycle_op_8d(sys)?,
            0x8e => self.cycle_op_8e(sys)?,
            0x8f => self.cycle_op_8f(sys)?,
            0x90 => self.cycle_op_90(sys)?,
            0x91 => self.cycle_op_91(sys)?,
            0x92 => self.cycle_op_92(sys)?,
            0x93 => self.cycle_op_93(sys)?,
            0x94 => self.cycle_op_94(sys)?,
            0x95 => self.cycle_op_95(sys)?,
            0x96 => self.cycle_op_96(sys)?,
            0x97 => self.cycle_op_97(sys)?,
            0x98 => self.op_98(sys)?,
            0x99 => self.cycle_op_99(sys)?,
            0x9a => self.op_9a(sys)?,
            0x9b => self.cycle_op_9b(sys)?,
            0x9c => self.cycle_op_9c(sys)?,
            0x9d => self.cycle_op_9d(sys)?,
            0x9e => self.cycle_op_9e(sys)?,
            0x9f => self.cycle_op_9f(sys)?,
            0xa0 => self.op_a0(sys)?,
            0xa1 => self.cycle_op_a1(sys)?,
            0xa2 => self.op_a2(sys)?,
            0xa3 => self.cycle_op_a3(sys)?,
            0xa4 => self.cycle_op_a4(sys)?,
            0xa5 => self.cycle_op_a5(sys)?,
            0xa6 => self.cycle_op_a6(sys)?,
            0xa7 => self.cycle_op_a7(sys)?,
            0xa8 => self.op_a8(sys)?,
            0xa9 => self.op_a9(sys)?,
            0xaa => self.op_aa(sys)?,
            0xab => self.op_ab(sys)?,
            0xac => self.cycle_op_ac(sys)?,
            0xad => self.cycle_op_ad(sys)?,
            0xae => self.cycle_op_ae(sys)?,
            0xaf => self.cycle_op_af(sys)?,
            0xb0 => self.cycle_op_b0(sys)?,
            0xb1 => self.cycle_op_b1(sys)?,
            0xb2 => self.cycle_op_b2(sys)?,
            0xb3 => self.cycle_op_b3(sys)?,
            0xb4 => self.cycle_op_b4(sys)?,
            0xb5 => self.cycle_op_b5(sys)?,
            0xb6 => self.cycle_op_b6(sys)?,
            0xb7 => self.cycle_op_b7(sys)?,
            0xb8 => self.op_b8(sys)?,
            0xb9 => self.cycle_op_b9(sys)?,
            0xba => self.op_ba(sys)?,
            0xbb => self.cycle_op_bb(sys)?,
            0xbc => self.cycle_op_bc(sys)?,
            0xbd => self.cycle_op_bd(sys)?,
            0xbe => self.cycle_op_be(sys)?,
            0xbf => self.cycle_op_bf(sys)?,
            0xc0 => self.op_c0(sys)?,
            0xc1 => self.cycle_op_c1(sys)?,
            0xc2 => self.op_c2(sys)?,
            0xc3 => self.cycle_op_c3(sys)?,
            0xc4 => self.cycle_op_c4(sys)?,
            0xc5 => self.cycle_op_c5(sys)?,
            0xc6 => self.cycle_op_c6(sys)?,
            0xc7 => self.cycle_op_c7(sys)?,
            0xc8 => self.op_c8(sys)?,
            0xc9 => self.op_c9(sys)?,
            0xca => self.op_ca(sys)?,
            0xcb => self.op_cb(sys)?,
            0xcc => self.cycle_op_cc(sys)?,
            0xcd => self.cycle_op_cd(sys)?,
            0xce => self.cycle_op_ce(sys)?,
            0xcf => self.cycle_op_cf(sys)?,
            0xd0 => self.cycle_op_d0(sys)?,
            0xd1 => self.cycle_op_d1(sys)?,
            0xd2 => self.cycle_op_d2(sys)?,
            0xd3 => self.cycle_op_d3(sys)?,
            0xd4 => self.cycle_op_d4(sys)?,
            0xd5 => self.cycle_op_d5(sys)?,
            0xd6 => self.cycle_op_d6(sys)?,
            0xd7 => self.cycle_op_d7(sys)?,
            0xd8 => self.op_d8(sys)?,
            0xd9 => self.cycle_op_d9(sys)?,
            0xda => self.op_da(sys)?,
            0xdb => self.cycle_op_db(sys)?,
            0xdc => self.cycle_op_dc(sys)?,
            0xdd => self.cycle_op_dd(sys)?,
            0xde => self.cycle_op_de(sys)?,
            0xdf => self.cycle_op_df(sys)?,
            0xe0 => self.op_e0(sys)?,
            0xe1 => self.cycle_op_e1(sys)?,
            0xe2 => self.op_e2(sys)?,
            0xe3 => self.cycle_op_e3(sys)?,
            0xe4 => self.cycle_op_e4(sys)?,
            0xe5 => self.cycle_op_e5(sys)?,
            0xe6 => self.cycle_op_e6(sys)?,
            0xe7 => self.cycle_op_e7(sys)?,
            0xe8 => self.op_e8(sys)?,
            0xe9 => self.op_e9(sys)?,
            0xea => self.op_ea(sys)?,
            0xeb => self.op_eb(sys)?,
            0xec => self.cycle_op_ec(sys)?,
            0xed => self.cycle_op_ed(sys)?,
            0xee => self.cycle_op_ee(sys)?,
            0xef => self.cycle_op_ef(sys)?,
            0xf0 => self.cycle_op_f0(sys)?,
            0xf1 => self.cycle_op_f1(sys)?,
            0xf2 => self.cycle_op_f2(sys)?,
            0xf3 => self.cycle_op_f3(sys)?,
            0xf4 => self.cycle_op_f4(sys)?,
            0xf5 => self.cycle_op_f5(sys)?,
            0xf6 => self.cycle_op_f6(sys)?,
            0xf7 => self.cycle_op_f7(sys)?,
            0xf8 => self.op_f8(sys)?,
            0xf9 => self.cycle_op_f9(sys)?,
            0xfa => self.op_fa(sys)?,
            0xfb => self.cycle_op_fb(sys)?,
            0xfc => self.cycle_op_fc(sys)?,
            0xfd => self.cycle_op_fd(sys)?,
            0xfe => self.cycle_op_fe(sys)?,
            0xff => self.cycle_op_ff(sys)?,
            _ => unreachable!(),
        }
        Some(())
    }
}
