
pub mod instr {
    pub const SHORT_INSTRUCTION: u8 = 0x02;
    pub const VAR_INSTRUCTION: u8 = 0x03;
}

pub mod operand {
    pub const LARGE_CONSTANT: u8 = 0x00;
    pub const SMALL_CONSTANT: u8 = 0x01;
    pub const VARIABLE: u8 = 0x02;
    pub const OMITTED: u8 = 0x03;
}

pub mod opcode {
    // lolololol
}

