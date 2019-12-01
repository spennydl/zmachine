use crate::{
    bits::ZWord,
    constants::{
        instr::*,
        operand::*,
    },
};

bitstruct! {
    Opcode: u8 {
        instruction_type: OpInstructionType, Width = U2, Offset = U6
    },

    LongInstruction: u8 {
        arg1_type: Arg1Type, Width = U1, Offset = U6,
        arg2_type: Arg2Type, Width = U1, Offset = U5,
        opcode: LongOpcode, Width = U5, Offset = U0
    },

    ShortInstruction: u8 {
        arg_type: ArgType, Width = U2, Offset = U4,
        opcode: ShortOpcode, Width = U4, Offset = U0
    },

    VarInstruction: u16 {
        is_2op: Is2Op, Width = U1, Offset = U13,
        opcode: VarOpcode, Width = U5, Offset = U8,
        arg1_type: VarArg1, Width = U2, Offset = U6,
        arg2_type: VarArg2, Width = U2, Offset = U4,
        arg3_type: VarArg3, Width = U2, Offset = U2,
        arg4_type: VarArg4, Width = U2, Offset = U0
    },

    BranchLabel: u16 {
        invert: BranchInv, Width = U1, Offset = U15,
        offset: BranchOffset, Width = U1, Offset = U14,
        unsigned_value: BranchUnsignedValue, Width = U6, Offset = U8,
        sign: BranchSign, Width = U1, Offset = U14,
        signed_value: BranchSignedValue, Width = U13, Offset = U0
    }
}

trait InstrTypeProvider {
    fn instr_type(&self) -> InstructionType;
}

impl InstrTypeProvider for Opcode {
    fn instr_type(&self) -> InstructionType {
        match self.instruction_type.value_of() {
            SHORT_INSTRUCTION => InstructionType::Short,
            VAR_INSTRUCTION => InstructionType::Variable,
            _ => InstructionType::Long
        }
    }
}

#[derive(Debug)]
pub(crate) enum InstructionType {
    Long, Short, Variable, ZeroOps
}

#[derive(Debug)]
pub(crate) struct Instruction {
    pub(crate) opcode: u8,
    pub(crate) ty: InstructionType,
    pub(crate) ops: Vec<Operand>,
}

#[derive(Debug)]
pub(crate) enum Address {
    StackPointer,
    Local(u16),
    Global(u16),
    Word(u16),
}

impl Default for Address {
    fn default() -> Address {
        Address::StackPointer
    }
}

impl Address {
    pub(crate) fn of(addr: u16) -> Self {
        if addr == 0 {
            Address::StackPointer
        } else if addr < 0x10 {
            Address::Local(addr - 1)
        } else {
            Address::Global(addr - 0x10)
        } // there is no reason to create word addr this way
    }

    pub(crate) fn addr(&self) -> u16 {
        match self {
            Address::StackPointer => 0,
            Address::Local(v) => *v,
            Address::Global(v) => *v,
            Address::Word(v) => *v,
        }
    }
}

#[derive(Debug)]
pub(crate) enum Operand {
    SmallConstant(u16),
    Variable(Address),
    LargeConstant(u16),
}

impl Operand {
    pub(crate) fn value(&self) -> u16 {
        match self {
            Operand::SmallConstant(v) => *v,
            Operand::LargeConstant(v) => *v,
            Operand::Variable(v) => v.addr(),
        }
    }
}

impl Instruction {
    pub fn from_mem(mem: &[u8]) -> (Instruction, usize) {
        let mut offset = 0 as usize;
        let op = Opcode::new(mem[offset]);
        let mut operands: Vec<Operand> = vec![];
        println!("have opcode {:?}, value_of {}", op, op.instruction_type.value_of());

        match op.instr_type() {
            InstructionType::Long => {
                let instr = LongInstruction::new(mem[offset]);
                offset += 1;

                let val = mem[offset];
                offset += 1;
                if instr.arg1_type.is_set() {
                    operands.push(Operand::Variable(Address::of(val as u16)));
                } else {
                    operands.push(Operand::SmallConstant(val as u16));
                }

                let val = mem[offset];
                offset += 1;
                if instr.arg2_type.is_set() {
                    operands.push(Operand::Variable(Address::of(val as u16)));
                } else {
                    operands.push(Operand::SmallConstant(val as u16));
                }

                (Instruction {
                    opcode: instr.opcode.value_of(),
                    ty: InstructionType::Long,
                    ops: operands,
                }, offset)
            },
            InstructionType::ZeroOps |
            InstructionType::Short => {
                let instr = ShortInstruction::new(mem[offset]);
                offset += 1;

                let arg_type = instr.arg_type.value_of();
                if let Some((op, b)) = Instruction::extract_operand(&arg_type, &mem[offset..]) {
                    operands.push(op);
                    offset += b;
                }

                let mut instr_type = InstructionType::Short;
                if operands.len() == 0 {
                    instr_type = InstructionType::ZeroOps;
                }

                (Instruction {
                    opcode: instr.opcode.value_of(),
                    ty: instr_type,
                    ops: operands,
                }, offset)
            },
            InstructionType::Variable => {
                let raw = ZWord::from((mem[offset], mem[offset + 1]));
                offset += 2;
                let instr = VarInstruction::new(raw.into());

                let arg1_type = instr.arg1_type.value_of() as u8;
                if let Some((op, b)) = Instruction::extract_operand(&arg1_type, &mem[offset..]) {
                    operands.push(op);
                    offset += b;
                }

                let arg2_type = instr.arg2_type.value_of() as u8;
                if let Some((op, b)) = Instruction::extract_operand(&arg2_type, &mem[offset..]) {
                    operands.push(op);
                    offset += b;
                }

                let arg3_type = instr.arg3_type.value_of() as u8;
                if let Some((op, b)) = Instruction::extract_operand(&arg3_type, &mem[offset..]) {
                    operands.push(op);
                    offset += b;
                }

                let arg4_type = instr.arg4_type.value_of() as u8;
                if let Some((op, b)) = Instruction::extract_operand(&arg4_type, &mem[offset..]) {
                    operands.push(op);
                    offset += b;
                }

                /*
                let mut ty = InstructionType::Variable;
                if operands.len() == 2 {
                    ty = InstructionType::Long;
                }
                */

                (Instruction {
                    opcode: instr.opcode.value_of() as u8,
                    ty: InstructionType::Variable,
                    ops: operands,
                }, offset)
            }
        }
    }

    fn extract_operand(ty: &u8, mem: &[u8]) -> Option<(Operand, usize)> {
        match *ty {
            LARGE_CONSTANT => {
                let val: ZWord = (mem[0], mem[1]).into();

                Some((Operand::LargeConstant(val.into()), 2))
            },
            SMALL_CONSTANT => {
                let val: ZWord = (0, mem[0]).into();

                Some((Operand::SmallConstant(val.into()), 1))
            },
            VARIABLE => {
                let addr: ZWord = (0, mem[0]).into();

                Some((Operand::Variable(Address::of(addr.into())), 1))
            },
            OMITTED => None,
            v => {
                println!("warn: unexpected operand type {}", v);

                None
            }
        }
    }
}
