use crate::opcodes::{OPCODES_MAP, OpCode};
use std::fmt;

pub mod state;
pub mod instruction;

use crate::cpu::state::*;

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
                println!("{:04x} {:04x}", indirect_addr_hi, indirect_addr_lo);
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
                    self.eor(opcode);
                }
                0x46 | 0x4A | 0x4E | 0x56 | 0x5E => {
                    self.lsr(opcode);
                }
                0x48 => {
                    self.pha(opcode);
                }
                0x4C | 0x6C => {
                    self.jmp(opcode);
                }
                0x58 => {
                    self.cli(opcode);
                }
                0x60 => {
                    self.rts(opcode);
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
                    self.bvs(opcode);
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
                    self.bcc(opcode);
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
                    self.bcs(opcode);
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
                    self.bne(opcode);
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
                    self.beq(opcode);
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
            (FLAG_B, 'B'),
            (FLAG_DECIMAL, 'D'),
            (FLAG_INTERRUPT_DISABLE, 'I'),
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
mod test;