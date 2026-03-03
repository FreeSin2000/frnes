use crate::cpu::state::*;
use crate::opcodes::OpCode;

impl CPU {
    pub fn lda(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    pub fn ldx(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_x);
        self.advance_pc(opcode);
    }

    pub fn ldy(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        self.register_y = value;
        self.update_zero_and_negative_flags(self.register_y);
        self.advance_pc(opcode);
    }

    pub fn sta(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        self.mem_write(addr, self.register_a);
        self.advance_pc(opcode);
    }

    pub fn stx(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        self.mem_write(addr, self.register_x);
        self.advance_pc(opcode);
    }

    pub fn sty(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        self.mem_write(addr, self.register_y);
        self.advance_pc(opcode);
    }

    pub fn pha(&mut self, opcode: &OpCode) {
        let data = self.register_a;
        self.stack_push(data);
        self.advance_pc(opcode);
    }

    pub fn pla(&mut self, opcode: &OpCode) {
        self.register_a = self.stack_pop();
        self.advance_pc(opcode);
    }

    pub fn php(&mut self, opcode: &OpCode) {
        let data = self.status;
        self.stack_push(data);
        self.advance_pc(opcode);
    }

    pub fn plp(&mut self, opcode: &OpCode) {
        self.status = self.stack_pop();
        self.advance_pc(opcode);
    }

    pub fn tax(&mut self, opcode: &OpCode) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
        self.advance_pc(opcode);
    }

    pub fn tay(&mut self, opcode: &OpCode) {
        self.register_y = self.register_a;
        self.update_zero_and_negative_flags(self.register_y);
        self.advance_pc(opcode);
    }

    pub fn tsx(&mut self, opcode: &OpCode) {
        self.register_x = self.sp;
        self.update_zero_and_negative_flags(self.register_x);
        self.advance_pc(opcode);
    }

    pub fn txa(&mut self, opcode: &OpCode) {
        self.register_a = self.register_x;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    pub fn tya(&mut self, opcode: &OpCode) {
        self.register_a = self.register_y;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    pub fn txs(&mut self, opcode: &OpCode) {
        self.sp = self.register_x;
        self.update_zero_and_negative_flags(self.sp);
        self.advance_pc(opcode);
    }

    pub fn adc(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);
        let carry = if self.get_flag(FLAG_CARRY) { 1 } else { 0 };

        let a = self.register_a;
        let sum = (a as u16)
            .wrapping_add(value as u16)
            .wrapping_add(carry as u16);

        self.set_flag(FLAG_CARRY, sum > 0xFF);

        let result = sum as u8;
        let overflow = (a ^ result) & (value ^ result) & 0x80 != 0;
        self.set_flag(FLAG_OVERFLOW, overflow);

        self.register_a = result;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    pub fn sbc(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);
        let carry = if self.get_flag(FLAG_CARRY) { 1 } else { 0 };

        let a = self.register_a;
        let neg_value = (!value) as u16;

        let sum = (a as u16)
            .wrapping_add(neg_value)
            .wrapping_add(carry as u16);

        self.set_flag(FLAG_CARRY, sum > 0xFF);

        let result = sum as u8;
        let overflow = (a ^ value) & (a ^ result) & 0x80 != 0;
        self.set_flag(FLAG_OVERFLOW, overflow);

        self.register_a = result;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    pub fn and(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        self.register_a &= value;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    pub fn ora(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        self.register_a |= value;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    pub fn bit(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        let result = self.register_a & value;
        self.set_flag(FLAG_ZERO, result == 0);
        self.set_flag(FLAG_OVERFLOW, value & FLAG_OVERFLOW != 0);
        self.set_flag(FLAG_NEGATIVE, value & FLAG_NEGATIVE != 0);
        self.advance_pc(opcode);
    }

    pub fn cmp(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        let a = self.register_a;
        let result = a.wrapping_sub(value);
        self.set_flag(FLAG_CARRY, a >= value);
        self.update_zero_and_negative_flags(result);
        self.advance_pc(opcode);
    }

    pub fn cpx(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        let x = self.register_x;
        let result = x.wrapping_sub(value);
        self.set_flag(FLAG_CARRY, x >= value);
        self.update_zero_and_negative_flags(result);
        self.advance_pc(opcode);
    }

    pub fn cpy(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        let y = self.register_y;
        let result = y.wrapping_sub(value);
        self.set_flag(FLAG_CARRY, y >= value);
        self.update_zero_and_negative_flags(result);
        self.advance_pc(opcode);
    }

    pub fn asl(&mut self, opcode: &OpCode) {
        match &opcode.mode {
            AddressingMode::Accumulator => {
                let value = self.register_a;
                let result = value << 1;
                self.set_flag(FLAG_CARRY, value & FLAG_NEGATIVE != 0);
                self.update_zero_and_negative_flags(result);
                self.register_a = result;
            }
            _ => {
                let addr = self.get_operand_address(&opcode.mode);
                let value = self.mem_read(addr);

                let result = value << 1;
                self.set_flag(FLAG_CARRY, value & FLAG_NEGATIVE != 0);
                self.update_zero_and_negative_flags(result);
                self.mem_write(addr, result);
            }
        }
        self.advance_pc(opcode);
    }

    pub fn lsr(&mut self, opcode: &OpCode) {
        match &opcode.mode {
            AddressingMode::Accumulator => {
                let value = self.register_a;
                let result = value >> 1;
                self.set_flag(FLAG_CARRY, value & 0x01 != 0);
                self.update_zero_and_negative_flags(result);
                self.register_a = result;
            }
            _ => {
                let addr = self.get_operand_address(&opcode.mode);
                let value = self.mem_read(addr);

                let result = value >> 1;
                self.set_flag(FLAG_CARRY, value & 0x01 != 0);
                self.update_zero_and_negative_flags(result);
                self.mem_write(addr, result);
            }
        }
        self.advance_pc(opcode);
    }

    pub fn rol(&mut self, opcode: &OpCode) {
        match &opcode.mode {
            AddressingMode::Accumulator => {
                let value = self.register_a;
                let result = if self.get_flag(FLAG_CARRY) {
                    value << 1 | 1
                } else {
                    value << 1
                };
                self.set_flag(FLAG_CARRY, value & 0x80 != 0);
                self.update_zero_and_negative_flags(result);
                self.register_a = result;
            }
            _ => {
                let addr = self.get_operand_address(&opcode.mode);
                let value = self.mem_read(addr);

                let result = if self.get_flag(FLAG_CARRY) {
                    value << 1 | 1
                } else {
                    value << 1
                };
                self.set_flag(FLAG_CARRY, value & 0x80 != 0);
                self.update_zero_and_negative_flags(result);
                self.mem_write(addr, result);
            }
        }
        self.advance_pc(opcode);
    }

    pub fn ror(&mut self, opcode: &OpCode) {
        match &opcode.mode {
            AddressingMode::Accumulator => {
                let value = self.register_a;
                let result = if self.get_flag(FLAG_CARRY) {
                    value >> 1 | 0x80
                } else {
                    value >> 1
                };
                self.set_flag(FLAG_CARRY, value & 0x01 != 0);
                self.update_zero_and_negative_flags(result);
                self.register_a = result;
            }
            _ => {
                let addr = self.get_operand_address(&opcode.mode);
                let value = self.mem_read(addr);

                let result = if self.get_flag(FLAG_CARRY) {
                    value >> 1 | 0x80
                } else {
                    value >> 1
                };
                self.set_flag(FLAG_CARRY, value & 0x01 != 0);
                self.update_zero_and_negative_flags(result);
                self.mem_write(addr, result);
            }
        }
        self.advance_pc(opcode);
    }

    pub fn inc(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);
        let result = value.wrapping_add(1);
        self.update_zero_and_negative_flags(result);
        self.mem_write(addr, result);
        self.advance_pc(opcode);
    }

    pub fn dec(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);
        let result = value.wrapping_sub(1);
        self.update_zero_and_negative_flags(result);
        self.mem_write(addr, result);
        self.advance_pc(opcode);
    }

    pub fn inx(&mut self, opcode: &OpCode) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
        self.advance_pc(opcode);
    }

    pub fn iny(&mut self, opcode: &OpCode) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
        self.advance_pc(opcode);
    }

    pub fn dex(&mut self, opcode: &OpCode) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
        self.advance_pc(opcode);
    }

    pub fn dey(&mut self, opcode: &OpCode) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
        self.advance_pc(opcode);
    }

    pub fn sec(&mut self, opcode: &OpCode) {
        self.set_flag(FLAG_CARRY, true);
        self.advance_pc(opcode);
    }

    pub fn bpl(&mut self, opcode: &OpCode) {
        let offset = self.mem_read(self.program_counter) as i8;
        self.advance_pc(opcode);
        let result = self
            .program_counter
            .wrapping_add(offset as i8 as i16 as u16);
        if !self.get_flag(FLAG_NEGATIVE) {
            self.program_counter = result;
        }
    }

    pub fn bmi(&mut self, opcode: &OpCode) {
        let offset = self.mem_read(self.program_counter) as i8;
        self.advance_pc(opcode);
        let result = self
            .program_counter
            .wrapping_add(offset as i8 as i16 as u16);
        if self.get_flag(FLAG_NEGATIVE) {
            self.program_counter = result;
        }
    }

    pub fn clc(&mut self, opcode: &OpCode) {
        self.set_flag(FLAG_CARRY, false);
        self.advance_pc(opcode);
    }

    pub fn jsr(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        self.advance_pc(opcode);
        self.stack_push_u16(self.program_counter.wrapping_sub(1));
        self.program_counter = addr;
    }

    pub fn rti(&mut self, opcode: &OpCode) {
        self.status = self.stack_pop();
        self.advance_pc(opcode);
        self.program_counter = self.stack_pop_u16();
    }

    pub fn eor(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        self.register_a = self.register_a ^ value;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    pub fn jmp(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        println!("{:04x}", addr);
        self.program_counter = addr;
    }

    pub fn cli(&mut self, opcode: &OpCode) {
        self.set_flag(FLAG_INTERRUPT_DISABLE, false);
        self.advance_pc(opcode);
    }

    pub fn rts(&mut self, opcode: &OpCode) {
        self.program_counter = self.stack_pop_u16();
        self.advance_pc(opcode);
    }

    pub fn bvs(&mut self, opcode: &OpCode) {
        let offset = self.mem_read(self.program_counter) as i8;
        self.advance_pc(opcode);
        let result = self
            .program_counter
            .wrapping_add(offset as i8 as i16 as u16);
        if self.get_flag(FLAG_OVERFLOW) {
            self.program_counter = result;
        }
    }

    pub fn bcc(&mut self, opcode: &OpCode) {
        let offset = self.mem_read(self.program_counter) as i8;
        self.advance_pc(opcode);
        let result = self
            .program_counter
            .wrapping_add(offset as i8 as i16 as u16);
        if !self.get_flag(FLAG_CARRY) {
            self.program_counter = result;
        }
    }

    pub fn bcs(&mut self, opcode: &OpCode) {
        let offset = self.mem_read(self.program_counter) as i8;
        self.advance_pc(opcode);
        let result = self
            .program_counter
            .wrapping_add(offset as i8 as i16 as u16);
        if self.get_flag(FLAG_CARRY) {
            self.program_counter = result;
        }
    }

    pub fn bne(&mut self, opcode: &OpCode) {
        let offset = self.mem_read(self.program_counter) as i8;
        self.advance_pc(opcode);
        let result = self
            .program_counter
            .wrapping_add(offset as i8 as i16 as u16);
        if !self.get_flag(FLAG_ZERO) {
            self.program_counter = result;
        }
    }
    
    pub fn beq(&mut self, opcode: &OpCode) {
        let offset = self.mem_read(self.program_counter) as i8;
        self.advance_pc(opcode);
        let result = self
            .program_counter
            .wrapping_add(offset as i8 as i16 as u16);
        if self.get_flag(FLAG_ZERO) {
            self.program_counter = result;
        }
    }
}
