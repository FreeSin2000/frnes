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
fn test_bcc_branches_when_carry_clear() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0x18, // CLC clears carry
        0x90, 0x02, // BCC skip next LDA when carry clear
        0xa9, 0x00, // would execute if branch failed
        0xa9, 0x42, // branch target
        0x00,
    ]);
    cpu.reset();
    cpu.run();

    assert_eq!(cpu.register_a, 0x42);
}

#[test]
fn test_bcc_does_not_branch_when_carry_set() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0x38, // SEC sets carry
        0x90, 0x03, // BCC should not branch when carry is set
        0xa9, 0x66, // should execute because branch is inhibited
        0x00, // BRK to stop sequential path
        0xa9, 0x00, // branch target if it were taken
        0x00,
    ]);
    cpu.reset();
    cpu.run();

    assert_eq!(cpu.register_a, 0x66);
}

#[test]
fn test_bcs_branches_when_carry_set() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0x38, // SEC sets carry flag
        0xb0, 0x02, // BCS skip next LDA when carry set
        0xa9, 0x00, 0xa9, 0x55, 0x00,
    ]);
    cpu.reset();
    cpu.run();

    assert_eq!(cpu.register_a, 0x55);
}

#[test]
fn test_bcs_does_not_branch_when_carry_clear() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0x18, // CLC clears carry
        0xb0, 0x02, // BCS should not branch when carry clear
        0xa9, 0x77, // should execute sequentially
        0x00, 0xa9, 0x00, // branch target if it were taken
        0x00,
    ]);
    cpu.reset();
    cpu.run();

    assert_eq!(cpu.register_a, 0x77);
}

#[test]
fn test_bvs_branches_when_overflow_set() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0x70, 0x02, // BVS skip next LDA when overflow set
        0xa9, 0x00, 0xa9, 0x99, 0x00,
    ]);
    cpu.reset();
    cpu.status |= FLAG_OVERFLOW;
    cpu.run();

    assert_eq!(cpu.register_a, 0x99);
}

#[test]
fn test_bvs_does_not_branch_when_overflow_clear() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0x70, 0x02, // BVS should not branch when overflow clear
        0xa9, 0x33, // executes if branch not taken
        0x00, 0xa9, 0x00, // branch target if taken
        0x00,
    ]);
    cpu.reset();
    cpu.status &= !FLAG_OVERFLOW;
    cpu.run();

    assert_eq!(cpu.register_a, 0x33);
}

#[test]
fn test_bne_branches_when_zero_clear() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0xa9, 0x01, // LDA #$01 -> zero flag clear
        0xd0, 0x03, // BNE skip next LDA when zero clear
        0xa9, 0x00, 0x00, 0xa9, 0x77, 0x00,
    ]);
    cpu.reset();
    cpu.run();

    assert_eq!(cpu.register_a, 0x77);
}

#[test]
fn test_bne_does_not_branch_when_zero_set() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0xa9, 0x00, // LDA #$00 -> sets zero flag
        0xd0, 0x02, // BNE should not branch when zero set
        0xa9, 0x66, // should execute sequentially
        0x00, 0xa9, 0x00, 0x00,
    ]);
    cpu.reset();
    cpu.run();

    assert_eq!(cpu.register_a, 0x66);
}

#[test]
fn test_beq_branches_when_zero_set() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0xa9, 0x00, // LDA #$00 -> zero flag set
        0xf0, 0x02, // BEQ skip next LDA when zero set
        0xa9, 0x11, 0xa9, 0x88, 0x00,
    ]);
    cpu.reset();
    cpu.run();

    assert_eq!(cpu.register_a, 0x88);
}

#[test]
fn test_beq_does_not_branch_when_zero_clear() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0xa9, 0x01, // LDA #$01 -> zero clear
        0xf0, 0x02, // BEQ should not branch when zero clear
        0xa9, 0x22, 0x00, 0xa9, 0x99, 0x00,
    ]);
    cpu.reset();
    cpu.run();

    assert_eq!(cpu.register_a, 0x22);
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
fn test_rts_pops_return_address_and_resumes() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0x20, 0x06, 0x80, // 0x8000: JSR $8006
        0xA9, 0x99, // 0x8003: RTS 应该跳回到这里！(JSR 结尾 $8002 + 1)
        0x00, // 0x8005: 最终在这里结束
        0xA9, 0x77, // 0x8006: 子程序开始: LDA #$77
        0x60, // 0x8008: RTS
    ]);
    cpu.reset();
    cpu.run();

    // After RTS returns, PC should end at BRK (0x8003) and A should hold 0x99
    assert_eq!(cpu.register_a, 0x99);
    assert_eq!(cpu.program_counter, 0x8005 + 1);
    // Stack pointer should be back to reset value
    assert_eq!(cpu.sp, 0xfd);
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
        0x4c, 0x04, 0x80, // JMP $8005
        0x00, // would run if JMP failed
        0xa9, 0x99, 0x00,
    ]);
    cpu.reset();
    cpu.run();

    assert_eq!(cpu.register_a, 0x99);
    assert_eq!(cpu.program_counter, 0x8007);
}

#[test]
fn test_jmp_indirect_wraparound_bug() {
    let mut cpu = CPU::new();

    cpu.load(vec![
        0x6C, 0xFF, 0x02, // JMP ($02FF) -> hardware bug wraps high byte to $0200
        0x00,
    ]);
    cpu.reset();

    // Arrange pointer bytes after program loading so they won't be overwritten
    cpu.mem_write(0x02FF, 0x34); // low byte of target
    cpu.mem_write(0x0200, 0x12); // high byte due to wraparound bug
    cpu.mem_write(0x0300, 0xAB); // would be used by a "correct" implementation

    cpu.run();

    assert_eq!(cpu.program_counter, 0x1234 + 1);
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
    assert_eq!(
        cpu.status & !(FLAG_ZERO | FLAG_NEGATIVE),
        0b1010_1010 & !(FLAG_ZERO | FLAG_NEGATIVE)
    );
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
