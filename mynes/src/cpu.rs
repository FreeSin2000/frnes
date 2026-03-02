use crate::opcodes::{self, OPCODES_MAP, OpCode};
use std::collections::HashMap;
use std::fmt;

const FLAG_CARRY: u8 = 0b0000_0001;
const FLAG_ZERO: u8 = 0b0000_0010;
const FLAG_NEGATIVE: u8 = 0b1000_0000;
const FLAG_OVERFLOW: u8 = 0b0100_0000;

const STACK_BASE: u16 = 0x0100;
pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    pub sp: u8,
    memory: [u8; 0xFFFF],
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect,
    Indirect_X,
    Indirect_Y,
    Accumulator,
    Relative,
    NoneAddressing,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0,
            program_counter: 0,
            sp: 0xFF,
            memory: [0; 0xFFFF],
        }
    }

    fn get_operand_address(&mut self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,

            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,

            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),

            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            }

            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            }

            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            }

            AddressingMode::Indirect => {
                let base = self.mem_read_u16(self.program_counter);
                let ptr = base & 0xFF00;
                let indirect_addr_lo = self.mem_read(base);
                let indirect_addr_hi = self.mem_read(ptr + ((base as u8).wrapping_add(1) as u16));
                (indirect_addr_hi as u16) << 8 | indirect_addr_lo as u16
            }

            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.program_counter);

                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }

            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.program_counter);

                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                deref
            }

            AddressingMode::NoneAddressing => {
                panic!("mode {:?} is not supported", mode);
            }
            AddressingMode::Accumulator => {
                panic!("mode {:?} does not use memory addressing", mode);
            }
            AddressingMode::Relative => {
                panic!("mode {:?} should be handled by branch logic", mode);
            }
        }
    }

    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }

    fn mem_read_u16(&mut self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16;
        (hi << 8) | (lo as u16)
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = 0;
        self.sp = 0xFD;

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run()
    }

    fn stack_top(&self) -> u16 {
        (self.sp as u16) | STACK_BASE
    }

    fn stack_push(&mut self, data: u8) {
        let addr = self.stack_top();
        self.mem_write(addr, data);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn stack_pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        let addr = self.stack_top();
        let data = self.mem_read(addr);
        data
    }

    fn stack_push_u16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xFF) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        hi << 8 | lo
    }

    fn set_flag(&mut self, bit: u8, flag: bool) {
        self.status = if flag {
            self.status | bit
        } else {
            self.status & (!bit)
        }
    }

    fn get_flag(&self, bit: u8) -> bool {
        self.status & bit != 0
    }

    fn compare(&mut self, reg: u8, oprand: u8) {
        self.set_flag(FLAG_CARRY, reg >= oprand);
        self.set_flag(FLAG_ZERO, reg == oprand);
        self.set_flag(FLAG_NEGATIVE, reg < oprand);
    }

    fn branch_if(&mut self, cond: bool, offset: i8) {
        if cond {
            let jmp_addr = self.program_counter.wrapping_add(offset as i16 as u16);
            self.program_counter = jmp_addr;
        }
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        self.set_flag(FLAG_ZERO, result == 0);
        self.set_flag(FLAG_NEGATIVE, result & 0b1000_0000 != 0);
    }

    fn advance_pc(&mut self, opcode: &OpCode) {
        self.program_counter = self.program_counter.wrapping_add((opcode.len - 1) as u16);
    }

    fn lda(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    fn ldx(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_x);
        self.advance_pc(opcode);
    }

    fn ldy(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        self.register_y = value;
        self.update_zero_and_negative_flags(self.register_y);
        self.advance_pc(opcode);
    }

    fn sta(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        self.mem_write(addr, self.register_a);
        self.advance_pc(opcode);
    }

    fn stx(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        self.mem_write(addr, self.register_x);
        self.advance_pc(opcode);
    }

    fn sty(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        self.mem_write(addr, self.register_y);
        self.advance_pc(opcode);
    }

    fn pha(&mut self, opcode: &OpCode) {
        let data = self.register_a;
        self.stack_push(data);
        self.advance_pc(opcode);
    }

    fn pla(&mut self, opcode: &OpCode) {
        self.register_a = self.stack_pop();
        self.advance_pc(opcode);
    }

    fn php(&mut self, opcode: &OpCode) {
        let data = self.status;
        self.stack_push(data);
        self.advance_pc(opcode);
    }

    fn plp(&mut self, opcode: &OpCode) {
        self.status = self.stack_pop();
        self.advance_pc(opcode);
    }

    fn tax(&mut self, opcode: &OpCode) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
        self.advance_pc(opcode);
    }

    fn tay(&mut self, opcode: &OpCode) {
        self.register_y = self.register_a;
        self.update_zero_and_negative_flags(self.register_y);
        self.advance_pc(opcode);
    }

    fn tsx(&mut self, opcode: &OpCode) {
        self.register_x = self.sp;
        self.update_zero_and_negative_flags(self.register_x);
        self.advance_pc(opcode);
    }

    fn txa(&mut self, opcode: &OpCode) {
        self.register_a = self.register_x;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    fn tya(&mut self, opcode: &OpCode) {
        self.register_a = self.register_y;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    fn txs(&mut self, opcode: &OpCode) {
        self.sp = self.register_x;
        self.update_zero_and_negative_flags(self.sp);
        self.advance_pc(opcode);
    }

    fn adc(&mut self, opcode: &OpCode) {
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

    fn sbc(&mut self, opcode: &OpCode) {
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

    fn and(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        self.register_a &= value;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    fn ora(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        self.register_a |= value;
        self.update_zero_and_negative_flags(self.register_a);
        self.advance_pc(opcode);
    }

    fn bit(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        let result = self.register_a & value;
        self.set_flag(FLAG_ZERO, result == 0);
        self.set_flag(FLAG_OVERFLOW, value & FLAG_OVERFLOW != 0);
        self.set_flag(FLAG_NEGATIVE, value & FLAG_NEGATIVE != 0);
        self.advance_pc(opcode);
    }

    fn cmp(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        let a = self.register_a;
        let result = a.wrapping_sub(value);
        self.set_flag(FLAG_CARRY, a >= value);
        self.update_zero_and_negative_flags(result);
        self.advance_pc(opcode);
    }

    fn cpx(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        let x = self.register_x;
        let result = x.wrapping_sub(value);
        self.set_flag(FLAG_CARRY, x >= value);
        self.update_zero_and_negative_flags(result);
        self.advance_pc(opcode);
    }

    fn cpy(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);

        let y = self.register_y;
        let result = y.wrapping_sub(value);
        self.set_flag(FLAG_CARRY, y >= value);
        self.update_zero_and_negative_flags(result);
        self.advance_pc(opcode);
    }

    fn asl(&mut self, opcode: &OpCode) {
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

    fn lsr(&mut self, opcode: &OpCode) {
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

    fn rol(&mut self, opcode: &OpCode) {
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

    fn ror(&mut self, opcode: &OpCode) {
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

    fn inc(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);
        let result = value.wrapping_add(1);
        self.update_zero_and_negative_flags(result);
        self.mem_write(addr, result);
        self.advance_pc(opcode);
    }

    fn dec(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        let value = self.mem_read(addr);
        let result = value.wrapping_sub(1);
        self.update_zero_and_negative_flags(result);
        self.mem_write(addr, result);
        self.advance_pc(opcode);
    }

    fn inx(&mut self, opcode: &OpCode) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
        self.advance_pc(opcode);
    }

    fn iny(&mut self, opcode: &OpCode) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
        self.advance_pc(opcode);
    }

    fn dex(&mut self, opcode: &OpCode) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
        self.advance_pc(opcode);
    }

    fn dey(&mut self, opcode: &OpCode) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
        self.advance_pc(opcode);
    }

    fn sec(&mut self, opcode: &OpCode) {
        self.set_flag(FLAG_CARRY, true);
        self.advance_pc(opcode);
    }

    fn bpl(&mut self, opcode: &OpCode) {
        let offset = self.mem_read(self.program_counter) as i8;
        self.advance_pc(opcode);
        let result = self
            .program_counter
            .wrapping_add(offset as i8 as i16 as u16);
        if !self.get_flag(FLAG_NEGATIVE) {
            self.program_counter = result;
        }
    }

    fn bmi(&mut self, opcode: &OpCode) {
        let offset = self.mem_read(self.program_counter) as i8;
        self.advance_pc(opcode);
        let result = self
            .program_counter
            .wrapping_add(offset as i8 as i16 as u16);
        if self.get_flag(FLAG_NEGATIVE) {
            self.program_counter = result;
        }
    }

    fn clc(&mut self, opcode: &OpCode) {
        self.set_flag(FLAG_CARRY, false);
        self.advance_pc(opcode);
    }

    fn jsr(&mut self, opcode: &OpCode) {
        let addr = self.get_operand_address(&opcode.mode);
        self.advance_pc(opcode);
        self.stack_push_u16(self.program_counter.wrapping_sub(1));
        self.program_counter = addr;
    }

    fn rti(&mut self, opcode: &OpCode) {
        self.status = self.stack_pop();
        self.program_counter = self.stack_pop_u16();
    }

    pub fn run(&mut self) {
        loop {
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;

            let opcode = OPCODES_MAP
                .get(&code)
                .expect(&format!("OpCode {:x} is not recognized", code));
            match code {
                0x00 => {
                    return;
                }
                0x01 | 0x05 | 0x09 | 0x0D | 0x11 | 0x15 | 0x19 | 0x1D => {
                    self.ora(opcode);
                }
                0x06 | 0x0A | 0x0E | 0x16 | 0x1E => {
                    self.asl(opcode);
                }
                0x08 => {
                    self.php(opcode);
                }
                0x10 => {
                    self.bpl(opcode);
                }
                0x18 => {
                    self.clc(opcode);
                }
                0x20 => {
                    self.jsr(opcode);
                }
                0x21 | 0x25 | 0x29 | 0x2D | 0x31 | 0x35 | 0x39 | 0x3D => {
                    self.and(opcode);
                }
                0x24 | 0x2C => {
                    self.bit(opcode);
                }
                0x26 | 0x2A | 0x2E | 0x36 | 0x3E => {
                    self.rol(opcode);
                }
                0x28 => {
                    self.plp(opcode);
                }
                0x30 => {
                    self.bmi(opcode);
                }
                0x38 => {
                    self.sec(opcode);
                }
                0x40 => {
                    self.rti(opcode);
                }
                0x41 | 0x45 | 0x49 | 0x4D | 0x51 | 0x55 | 0x59 | 0x5D => {
                    todo!("EOR");
                }
                0x46 | 0x4A | 0x4E | 0x56 | 0x5E => {
                    self.lsr(opcode);
                }
                0x48 => {
                    self.pha(opcode);
                }
                0x4C | 0x6C => {
                    todo!("JMP");
                }
                0x58 => {
                    todo!("CLI");
                }
                0x60 => {
                    todo!("RTS");
                }
                0x61 | 0x65 | 0x69 | 0x6D | 0x71 | 0x75 | 0x79 | 0x7D => {
                    self.adc(opcode);
                }
                0x66 | 0x6A | 0x6E | 0x76 | 0x7E => {
                    self.ror(opcode);
                }
                0x68 => {
                    self.pla(opcode);
                }
                0x70 => {
                    todo!("BVS");
                }
                0x78 => {
                    todo!("SEI");
                }
                0x81 | 0x85 | 0x8D | 0x91 | 0x95 | 0x99 | 0x9D => {
                    self.sta(opcode);
                }
                0x84 | 0x8C | 0x94 => {
                    self.sty(opcode);
                }
                0x86 | 0x8E | 0x96 => {
                    self.stx(opcode);
                }
                0x88 => {
                    self.dey(opcode);
                }
                0x8A => {
                    self.txa(opcode);
                }
                0x90 => {
                    todo!("BCC");
                }
                0x98 => {
                    self.tya(opcode);
                }
                0x9A => {
                    self.txs(opcode);
                }
                0xA0 | 0xA4 | 0xAC | 0xB4 | 0xBC => {
                    self.ldy(opcode);
                }
                0xA1 | 0xA5 | 0xA9 | 0xAD | 0xB1 | 0xB5 | 0xB9 | 0xBD => {
                    self.lda(opcode);
                }
                0xA2 | 0xA6 | 0xAE | 0xB6 | 0xBE => {
                    self.ldx(opcode);
                }
                0xA8 => {
                    self.tay(opcode);
                }
                0xAA => {
                    self.tax(opcode);
                }
                0xB0 => {
                    todo!("BCS");
                }
                0xB8 => {
                    todo!("CLV");
                }
                0xBA => {
                    self.tsx(opcode);
                }
                0xC0 | 0xC4 | 0xCC => {
                    self.cpy(opcode);
                }
                0xC1 | 0xC5 | 0xC9 | 0xCD | 0xD1 | 0xD5 | 0xD9 | 0xDD => {
                    self.cmp(opcode);
                }
                0xC6 | 0xCE | 0xD6 | 0xDE => {
                    self.dec(opcode);
                }
                0xC8 => {
                    self.iny(opcode);
                }
                0xCA => {
                    self.dex(opcode);
                }
                0xD0 => {
                    todo!("BNE");
                }
                0xD8 => {
                    todo!("CLD");
                }
                0xE0 | 0xE4 | 0xEC => {
                    self.cpx(opcode);
                }
                0xE1 | 0xE5 | 0xE9 | 0xED | 0xF1 | 0xF5 | 0xF9 | 0xFD => {
                    self.sbc(opcode);
                }
                0xE6 | 0xEE | 0xF6 | 0xFE => {
                    self.inc(opcode);
                }
                0xE8 => {
                    self.inx(opcode);
                }
                0xEA => {
                    todo!("NOP");
                }
                0xF0 => {
                    todo!("BEQ");
                }
                0xF8 => {
                    todo!("SED");
                }
                _ => todo!(),
            }
            println!("{}", self);
        }
    }
}

impl fmt::Display for CPU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let flags = [
            (FLAG_NEGATIVE, 'N'),
            (FLAG_OVERFLOW, 'V'),
            (0b0010_0000, '-'), // unused bit
            (0b0001_0000, 'B'),
            (0b0000_1000, 'D'),
            (0b0000_0100, 'I'),
            (FLAG_ZERO, 'Z'),
            (FLAG_CARRY, 'C'),
        ];
        let status_repr: String = flags
            .iter()
            .map(|(mask, label)| if self.status & mask != 0 { *label } else { '-' })
            .collect();

        write!(
            f,
            "PC=0x{:04X}  A=0x{:02X}  X=0x{:02X}  Y=0x{:02X}  SP=0x{:02X}  STATUS=[{}]",
            self.program_counter, self.register_a, self.register_x, self.register_y, self.sp, status_repr
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_reset_initializes_stack_pointer() {
        let mut cpu = CPU::new();
        cpu.reset();

        assert_eq!(cpu.sp, 0xfd, "6502 reset should position SP at 0xFD");
    }

    #[test]
    fn test_stack_push_and_pop_roundtrip() {
        let mut cpu = CPU::new();
        cpu.reset();
        let initial_sp = cpu.sp;

        cpu.stack_push(0xAB);

        let expected_addr = STACK_BASE | initial_sp as u16;
        assert_eq!(cpu.mem_read(expected_addr), 0xAB);
        assert_eq!(cpu.sp, initial_sp.wrapping_sub(1));

        let popped = cpu.stack_pop();
        assert_eq!(popped, 0xAB);
        assert_eq!(cpu.sp, initial_sp);
    }

    #[test]
    fn test_stack_push_and_pop_u16_roundtrip() {
        let mut cpu = CPU::new();
        cpu.reset();
        let initial_sp = cpu.sp;

        cpu.stack_push_u16(0xABCD);

        assert_eq!(cpu.sp, initial_sp.wrapping_sub(2));
        assert_eq!(cpu.mem_read(STACK_BASE | initial_sp as u16), 0xAB);
        assert_eq!(
            cpu.mem_read(STACK_BASE | initial_sp.wrapping_sub(1) as u16),
            0xCD
        );

        let value = cpu.stack_pop_u16();
        assert_eq!(value, 0xABCD);
        assert_eq!(cpu.sp, initial_sp);
    }

    #[test]
    fn test_flag_helpers_control_status_register() {
        let mut cpu = CPU::new();
        cpu.status = 0;

        cpu.set_flag(FLAG_CARRY, true);
        assert!(cpu.get_flag(FLAG_CARRY));

        cpu.set_flag(FLAG_ZERO, true);
        assert!(cpu.get_flag(FLAG_ZERO));

        cpu.set_flag(FLAG_CARRY, false);
        assert!(!cpu.get_flag(FLAG_CARRY));
        assert!(cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_compare_updates_carry_zero_and_negative() {
        let mut cpu = CPU::new();

        cpu.status = 0;
        cpu.compare(0x10, 0x0F);
        assert!(cpu.get_flag(FLAG_CARRY));
        assert!(!cpu.get_flag(FLAG_ZERO));
        assert!(!cpu.get_flag(FLAG_NEGATIVE));

        cpu.status = 0;
        cpu.compare(0x10, 0x10);
        assert!(cpu.get_flag(FLAG_CARRY));
        assert!(cpu.get_flag(FLAG_ZERO));
        assert!(!cpu.get_flag(FLAG_NEGATIVE));

        cpu.status = 0;
        cpu.compare(0x10, 0x11);
        assert!(!cpu.get_flag(FLAG_CARRY));
        assert!(!cpu.get_flag(FLAG_ZERO));
        assert!(cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_branch_if_condition_true_moves_forward() {
        let mut cpu = CPU::new();
        cpu.program_counter = 0x8000;

        cpu.branch_if(true, 6);

        assert_eq!(cpu.program_counter, 0x8006);
    }

    #[test]
    fn test_branch_if_condition_true_moves_backward() {
        let mut cpu = CPU::new();
        cpu.program_counter = 0x8050;

        cpu.branch_if(true, -0x10);

        assert_eq!(cpu.program_counter, 0x8040);
    }

    #[test]
    fn test_branch_if_condition_false_keeps_program_counter() {
        let mut cpu = CPU::new();
        cpu.program_counter = 0x9000;

        cpu.branch_if(false, 0x7F);

        assert_eq!(cpu.program_counter, 0x9000);
    }

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register_a, 0x05);
        assert!(cpu.status & 0b0000_0010 == 0b00);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xaa, 0x00]);
        cpu.reset();
        cpu.register_a = 10;
        cpu.run();

        assert_eq!(cpu.register_x, 10)
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xe8, 0xe8, 0x00]);
        cpu.reset();
        cpu.register_x = 0xff;
        cpu.run();
        assert_eq!(cpu.register_x, 1)
    }
    #[test]
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);

        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_ldx_immediate_sets_zero_flag() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa2, 0x00, 0x00]);

        assert_eq!(cpu.register_x, 0x00);
        assert!(cpu.get_flag(FLAG_ZERO));
        assert!(!cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_ldy_absolute_sets_negative_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x1234, 0x80);

        cpu.load_and_run(vec![0xac, 0x34, 0x12, 0x00]);

        assert_eq!(cpu.register_y, 0x80);
        assert!(cpu.get_flag(FLAG_NEGATIVE));
        assert!(!cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_store_instructions_write_memory() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x8d, 0x00, 0x20, 0x86, 0x10, 0x8c, 0x02, 0x20, 0x00]);
        cpu.reset();
        cpu.register_a = 0x3c;
        cpu.register_x = 0x77;
        cpu.register_y = 0x55;
        cpu.run();

        assert_eq!(cpu.mem_read(0x2000), 0x3c);
        assert_eq!(cpu.mem_read(0x0010), 0x77);
        assert_eq!(cpu.mem_read(0x2002), 0x55);
    }

    #[test]
    fn test_transfer_instructions_update_registers_and_flags() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa9, 0x00, 0xa8, 0xa9, 0x10, 0xaa, 0x8a, 0x98, 0x00]);

        assert_eq!(cpu.register_y, 0x00);
        assert_eq!(cpu.register_x, 0x10);
        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_php_plp_roundtrip_status_register() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x08, 0x28, 0x00]);
        cpu.reset();
        cpu.status = 0b1010_1100;
        cpu.run();

        assert_eq!(cpu.status, 0b1010_1100);
        assert_eq!(cpu.sp, 0xfd);
    }

    #[test]
    fn test_pha_pla_roundtrip_accumulator() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x48, 0x68, 0x00]);
        cpu.reset();
        cpu.register_a = 0x7f;
        cpu.run();

        assert_eq!(cpu.register_a, 0x7f);
        assert_eq!(cpu.sp, 0xfd);
    }

    #[test]
    fn test_ora_immediate_sets_negative_flag() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa9, 0x40, 0x09, 0xc0, 0x00]);

        assert_eq!(cpu.register_a, 0xC0);
        assert!(cpu.get_flag(FLAG_NEGATIVE));
        assert!(!cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_and_immediate_sets_zero_flag() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa9, 0x0f, 0x29, 0xf0, 0x00]);

        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_bit_absolute_updates_negative_and_overflow() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x2000, 0b1100_0000);

        cpu.load_and_run(vec![0xa9, 0xff, 0x2c, 0x00, 0x20, 0x00]);

        assert!(cpu.get_flag(FLAG_OVERFLOW));
        assert!(cpu.get_flag(FLAG_NEGATIVE));
        assert!(!cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_adc_sets_overflow_flag() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xa9, 0x50, 0x69, 0x50, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_a, 0xa0);
        assert!(cpu.get_flag(FLAG_OVERFLOW));
        assert!(!cpu.get_flag(FLAG_CARRY));
    }

    #[test]
    fn test_sbc_uses_carry_as_borrow() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xa9, 0x50, 0xe9, 0x10, 0x00]);
        cpu.reset();
        cpu.status = FLAG_CARRY; // no borrow
        cpu.run();

        assert_eq!(cpu.register_a, 0x40);
        assert!(cpu.get_flag(FLAG_CARRY));
        assert!(!cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_cmp_sets_zero_and_carry() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x0010, 0x55);

        cpu.load_and_run(vec![0xa9, 0x55, 0xc5, 0x10, 0x00]);

        assert!(cpu.get_flag(FLAG_ZERO));
        assert!(cpu.get_flag(FLAG_CARRY));
    }

    #[test]
    fn test_cpx_sets_negative_flag() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa2, 0x10, 0xe0, 0x20, 0x00]);

        assert!(cpu.get_flag(FLAG_NEGATIVE));
        assert!(!cpu.get_flag(FLAG_CARRY));
    }

    #[test]
    fn test_cpy_sets_zero_flag() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa0, 0x20, 0xc0, 0x20, 0x00]);

        assert!(cpu.get_flag(FLAG_ZERO));
        assert!(cpu.get_flag(FLAG_CARRY));
    }

    #[test]
    fn test_txs_and_tsx_roundtrip_stack_pointer() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa2, 0x7f, 0x9a, 0xba, 0x00]);

        assert_eq!(cpu.sp, 0x7f);
        assert_eq!(cpu.register_x, 0x7f);
    }

    #[test]
    fn test_asl_accumulator_sets_carry() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa9, 0x81, 0x0a, 0x00]);

        assert_eq!(cpu.register_a, 0x02);
        assert!(cpu.get_flag(FLAG_CARRY));
        assert!(!cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_asl_zeropage_updates_memory_and_zero_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x0042, 0x80);

        cpu.load_and_run(vec![0x06, 0x42, 0x00]);

        assert_eq!(cpu.mem_read(0x0042), 0x00);
        assert!(cpu.get_flag(FLAG_CARRY));
        assert!(cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_lsr_accumulator_clears_negative_sets_carry() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa9, 0x03, 0x4a, 0x00]);

        assert_eq!(cpu.register_a, 0x01);
        assert!(!cpu.get_flag(FLAG_NEGATIVE));
        assert!(cpu.get_flag(FLAG_CARRY));
    }

    #[test]
    fn test_rol_accumulator_uses_carry() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x38, 0xa9, 0x40, 0x2a, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_a, 0x81);
        assert!(!cpu.get_flag(FLAG_CARRY));
        assert!(cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_ror_zeropage_with_carry_in() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x0005, 0x02);

        cpu.load(vec![0x38, 0x66, 0x05, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.mem_read(0x0005), 0x81);
        assert!(!cpu.get_flag(FLAG_ZERO));
        assert!(!cpu.get_flag(FLAG_CARRY));
        assert!(cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_inc_and_dec_update_flags() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x00aa, 0xff);

        cpu.load_and_run(vec![0xe6, 0xaa, 0xc6, 0xaa, 0x00]);

        assert_eq!(cpu.mem_read(0x00aa), 0xff);
        assert!(!cpu.get_flag(FLAG_ZERO));
        assert!(cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_dex_underflow_sets_negative() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa2, 0x00, 0xca, 0x00]);

        assert_eq!(cpu.register_x, 0xff);
        assert!(cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_dey_sets_zero_flag() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa0, 0x01, 0x88, 0x00]);

        assert_eq!(cpu.register_y, 0x00);
        assert!(cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_bit_zero_flag_when_mask_clears_a() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x0040, 0x00);

        cpu.load_and_run(vec![0xa9, 0xff, 0x24, 0x40, 0x00]);

        assert!(cpu.get_flag(FLAG_ZERO));
        assert!(!cpu.get_flag(FLAG_OVERFLOW));
        assert!(!cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_adc_sets_carry_flag_on_overflow() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa9, 0xff, 0x69, 0x01, 0x00]);

        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.get_flag(FLAG_CARRY));
        assert!(cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_sbc_with_borrow_clears_carry_and_sets_negative() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xa9, 0x10, 0xe9, 0x20, 0x00]);
        cpu.reset();
        cpu.status = FLAG_CARRY; // ensure initial borrow clear
        cpu.run();

        assert_eq!(cpu.register_a, 0xf0);
        assert!(!cpu.get_flag(FLAG_CARRY));
        assert!(cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_lsr_zeropage_shifts_into_carry() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x0002, 0x01);

        cpu.load_and_run(vec![0x46, 0x02, 0x00]);

        assert_eq!(cpu.mem_read(0x0002), 0x00);
        assert!(cpu.get_flag(FLAG_CARRY));
        assert!(cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_rol_zeropage_incorporates_previous_carry() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x00f0, 0x7f);

        cpu.load(vec![0x38, 0x26, 0xf0, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.mem_read(0x00f0), 0xff);
        assert!(!cpu.get_flag(FLAG_CARRY));
        assert!(cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_iny_wraps_and_sets_zero_flag() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xa0, 0xff, 0xc8, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_y, 0x00);
        assert!(cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_bpl_branches_when_negative_clear() {
        let mut cpu = CPU::new();

        cpu.load(vec![
            0xa9, 0x01, // LDA #$01 (negative clear)
            0x10, 0x02, // BPL skip next instruction
            0xa9, 0x00, // would execute if branch failed
            0xa9, 0x42, // branch target
            0x00,
        ]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_a, 0x42);
    }

    #[test]
    fn test_bpl_does_not_branch_when_negative_set() {
        let mut cpu = CPU::new();

        cpu.load(vec![
            0xa9, 0xff, // LDA #$FF (sets negative)
            0x10, 0x02, // BPL would skip next LDA if not negative
            0xa9, 0x66, // should execute because branch is inhibited
            0x00,
        ]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_a, 0x66);
    }

    #[test]
    fn test_clc_clears_carry_flag() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x18, 0x00]);
        cpu.reset();
        cpu.status |= FLAG_CARRY;
        cpu.run();

        assert!(!cpu.get_flag(FLAG_CARRY));
    }

    #[test]
    fn test_jsr_pushes_return_address_and_jumps() {
        let mut cpu = CPU::new();

        // Program layout:
        // 0x8000: JSR $8005 -> push return addr (0x8002) then jump to 0x8005
        // 0x8003: BRK (should be skipped)
        // 0x8005: LDA #$42
        // 0x8007: BRK
        cpu.load(vec![0x20, 0x05, 0x80, 0x00, 0x00, 0xa9, 0x42, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_a, 0x42);
        // Stack should contain return address 0x8002 (hi byte first)
        assert_eq!(cpu.stack_pop_u16(), 0x8002);
    }

    #[test]
    fn test_bmi_branches_when_negative_set() {
        let mut cpu = CPU::new();

        cpu.load(vec![
            0xa9, 0xff, // LDA #$FF -> sets negative flag
            0x30, 0x02, // BMI skip next instruction
            0xa9, 0x00, // would clear A if branch failed
            0xa9, 0x77, // branch target
            0x00,
        ]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_a, 0x77);
    }

    #[test]
    fn test_bmi_does_not_branch_when_negative_clear() {
        let mut cpu = CPU::new();

        cpu.load(vec![
            0xa9, 0x01, // LDA #$01 -> negative clear
            0x30, 0x02, // BMI shouldn't branch
            0xa9, 0x66, // executed if branch skipped
            0x00,
        ]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_a, 0x66);
    }

    #[test]
    fn test_cli_clears_interrupt_disable() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x58, 0x00]);
        cpu.reset();
        cpu.status |= 0b0000_0100; // set I flag
        cpu.run();

        assert_eq!(cpu.status & 0b0000_0100, 0);
    }

    #[test]
    fn test_eor_immediate_sets_flags() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xa9, 0xF0, 0x49, 0xFF, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_a, 0x0F);
        assert!(!cpu.get_flag(FLAG_NEGATIVE));
        assert!(!cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_eor_absolute_sets_zero_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x2000, 0xAA);

        cpu.load(vec![0xa9, 0xAA, 0x4d, 0x00, 0x20, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_jmp_absolute_sets_program_counter() {
        let mut cpu = CPU::new();

        cpu.load(vec![
            0x4c, 0x05, 0x80, // JMP $8005
            0x00, // would run if JMP failed
            0xa9, 0x99, 0x00,
        ]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_a, 0x99);
    }

    #[test]
    fn test_jmp_indirect_wraparound_bug() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x80FF, 0x34);
        cpu.mem_write(0x8000, 0x12); // should be ignored due to bug

        cpu.load(vec![
            0x6C, 0xFF, 0x80, // JMP ($80FF) -> should read high byte from $8000
            0x00,
        ]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.program_counter, 0x1234);
    }

    #[test]
    fn test_rti_restores_status_and_resumes_execution() {
        let mut cpu = CPU::new();

        // Interrupt vector -> 0x8000
        cpu.load(vec![
            0x40, // RTI
            0x00, // BRK to stop after return target executes
            0xa9, 0x55, // LDA #$55 (this is where RTI should resume)
            0x00,
        ]);
        cpu.reset();

        // Simulate interrupt frame: hardware pushes PCH, PCL, then status
        cpu.stack_push(0x80); // PCH for 0x8002
        cpu.stack_push(0x02); // PCL for 0x8002
        cpu.stack_push(0b1010_1010);

        cpu.run();
        
        // Status should match what was restored, except bits modified by subsequent LDA
        assert_eq!(cpu.status & !(FLAG_ZERO | FLAG_NEGATIVE), 0b1010_1010 & !(FLAG_ZERO | FLAG_NEGATIVE));
        assert_eq!(cpu.register_a, 0x55);
        assert_eq!(cpu.program_counter, 0x8004 + 1); // PC should point past the LDA
    }

    #[test]
    fn test_cpu_display_formats_registers_and_flags() {
        let mut cpu = CPU::new();
        cpu.program_counter = 0xC123;
        cpu.register_a = 0x10;
        cpu.register_x = 0x20;
        cpu.register_y = 0x30;
        cpu.sp = 0x7F;
        cpu.status = FLAG_NEGATIVE | FLAG_OVERFLOW | 0b0001_0000 | FLAG_ZERO;

        let rendered = format!("{}", cpu);
        assert_eq!(
            rendered,
            "PC=0xC123  A=0x10  X=0x20  Y=0x30  SP=0x7F  STATUS=[NV-B--Z-]"
        );
    }
}
