use crate::opcodes::{self, OPCODES_MAP};
use std::collections::HashMap;

const FLAG_CARRY: u8 = 0b0000_0001;
const FLAG_ZERO: u8 = 0b0000_0010;
const FLAG_NEGATIVE: u8 = 0b1000_0000;
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

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
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
                    todo!("ORA");
                }
                0x06 | 0x0A | 0x0E | 0x16 | 0x1E => {
                    todo!("ASL");
                }
                0x08 => {
                    todo!("PHP");
                }
                0x10 => {
                    todo!("BPL");
                }
                0x18 => {
                    todo!("CLC");
                }
                0x20 => {
                    todo!("JSR");
                }
                0x21 | 0x25 | 0x29 | 0x2D | 0x31 | 0x35 | 0x39 | 0x3D => {
                    todo!("AND");
                }
                0x24 | 0x2C => {
                    todo!("BIT");
                }
                0x26 | 0x2A | 0x2E | 0x36 | 0x3E => {
                    todo!("ROL");
                }
                0x28 => {
                    todo!("PLP");
                }
                0x30 => {
                    todo!("BMI");
                }
                0x38 => {
                    todo!("SEC");
                }
                0x40 => {
                    todo!("RTI");
                }
                0x41 | 0x45 | 0x49 | 0x4D | 0x51 | 0x55 | 0x59 | 0x5D => {
                    todo!("EOR");
                }
                0x46 | 0x4A | 0x4E | 0x56 | 0x5E => {
                    todo!("LSR");
                }
                0x48 => {
                    todo!("PHA");
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
                    todo!("ADC");
                }
                0x66 | 0x6A | 0x6E | 0x76 | 0x7E => {
                    todo!("ROR");
                }
                0x68 => {
                    todo!("PLA");
                }
                0x70 => {
                    todo!("BVS");
                }
                0x78 => {
                    todo!("SEI");
                }
                0x81 | 0x85 | 0x8D | 0x91 | 0x95 | 0x99 | 0x9D => {
                    self.sta(&opcode.mode);
                }
                0x84 | 0x8C | 0x94 => {
                    todo!("STY");
                }
                0x86 | 0x8E | 0x96 => {
                    todo!("STX");
                }
                0x88 => {
                    todo!("DEY");
                }
                0x8A => {
                    todo!("TXA");
                }
                0x90 => {
                    todo!("BCC");
                }
                0x98 => {
                    todo!("TYA");
                }
                0x9A => {
                    todo!("TXS");
                }
                0xA0 | 0xA4 | 0xAC | 0xB4 | 0xBC => {
                    todo!("LDY");
                }
                0xA1 | 0xA5 | 0xA9 | 0xAD | 0xB1 | 0xB5 | 0xB9 | 0xBD => {
                    self.lda(&opcode.mode);
                }
                0xA2 | 0xA6 | 0xAE | 0xB6 | 0xBE => {
                    todo!("LDX");
                }
                0xA8 => {
                    todo!("TAY");
                }
                0xAA => {
                    self.tax();
                }
                0xB0 => {
                    todo!("BCS");
                }
                0xB8 => {
                    todo!("CLV");
                }
                0xBA => {
                    todo!("TSX");
                }
                0xC0 | 0xC4 | 0xCC => {
                    todo!("CPY");
                }
                0xC1 | 0xC5 | 0xC9 | 0xCD | 0xD1 | 0xD5 | 0xD9 | 0xDD => {
                    todo!("CMP");
                }
                0xC6 | 0xCE | 0xD6 | 0xDE => {
                    todo!("DEC");
                }
                0xC8 => {
                    todo!("INY");
                }
                0xCA => {
                    todo!("DEX");
                }
                0xD0 => {
                    todo!("BNE");
                }
                0xD8 => {
                    todo!("CLD");
                }
                0xE0 | 0xE4 | 0xEC => {
                    todo!("CPX");
                }
                0xE1 | 0xE5 | 0xE9 | 0xED | 0xF1 | 0xF5 | 0xF9 | 0xFD => {
                    todo!("SBC");
                }
                0xE6 | 0xEE | 0xF6 | 0xFE => {
                    todo!("INC");
                }
                0xE8 => {
                    self.inx();
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
            self.program_counter += (opcode.len - 1) as u16;
        }
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
}
