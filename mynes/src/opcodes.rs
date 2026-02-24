use crate::cpu::AddressingMode;
use std::collections::HashMap;
use std::sync::LazyLock;

macro_rules! define_opcodes {
    ($($code:expr, $mnemonic:expr, $len:expr, $cycles:expr, $mode:ident;)*) => {
        vec![
            $(
                OpCode::new($code, $mnemonic, $len, $cycles, AddressingMode::$mode),
            )*
        ]
    };
}
pub struct OpCode {
    pub code: u8,
    pub mnemonic: &'static str,
    pub len: u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}

impl OpCode {
    pub const fn new(
        code: u8,
        mnemonic: &'static str,
        len: u8,
        cycles: u8,
        mode: AddressingMode,
    ) -> Self {
        OpCode {
            code: code,
            mnemonic: mnemonic,
            len: len,
            cycles: cycles,
            mode: mode,
        }
    }
}

pub static CPU_OPS_CODES: LazyLock<Vec<OpCode>> = LazyLock::new(|| {
    define_opcodes!(
        0x00, "BRK", 1, 7, NoneAddressing;
        0xaa, "TAX", 1, 2, NoneAddressing;
        0xe8, "INX", 1, 2, NoneAddressing;
        0xa9, "LDA", 2, 2, Immediate;
        0xa5, "LDA", 2, 3, ZeroPage;
        0xb5, "LDA", 2, 4, ZeroPage_X;
        0xad, "LDA", 3, 4, Absolute;
        0xbd, "LDA", 3, 4, /*+1 if page crossed*/ Absolute_X;
        
        0xb9, "LDA", 3, 4, /*+1 if page crossed*/ Absolute_Y;
        0xa1, "LDA", 2, 6, Indirect_X;
        
        0xb1, "LDA", 2, 5, /*+1 if page crossed*/ Indirect_Y;

        0x85, "STA", 2, 3, ZeroPage;
        0x95, "STA", 2, 4, ZeroPage_X;
        0x8d, "STA", 3, 4, Absolute;
        0x9d, "STA", 3, 5, Absolute_X;
        0x99, "STA", 3, 5, Absolute_Y;
        0x81, "STA", 2, 6, Indirect_X;
        0x91, "STA", 2, 6, Indirect_Y;
    )
});

pub static OPCODES_MAP: LazyLock<HashMap<u8, &'static OpCode>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for cpuop in &*CPU_OPS_CODES {
        map.insert(cpuop.code, cpuop);
    }
    map
});
