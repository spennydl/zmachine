use crate::zmemory::ZMemory;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::cell::RefCell;

#[derive(Default, Debug)]
pub struct StackFrame {
    locals: Vec<u16>,
    stack: Vec<u16>,
    ret_addr: u8,
    pc: u16,
}

#[derive(Default, Debug)]
pub struct ZMachineState {
    machine_version: u8,
    stack: Vec<StackFrame>,
    current: RefCell<StackFrame>,
}

impl ZMachineState {
    pub fn reset(&mut self, header: &[u8]) {
        self.machine_version = header[0];

        self.stack = Vec::new();
        self.current.replace(StackFrame::default());

        let mut val = header[6] as u16;
        val = (val << 8) | header[7] as u16;
        self.current.get_mut().pc = val;
    }

    pub fn push_frame(&mut self, frame: StackFrame) {
        println!("Pushing frame: {:x?}", frame);
        self.stack.push(self.current.replace(frame));
    }
}

#[derive(Debug)]
pub(crate) enum Operand {
    SmallConstant(u8),
    Variable(u8),
    LargeConstant(u16),
}

impl Operand {
    #[inline]
    pub fn get_value(&self, state: &mut ZMachineState) -> i32 {
        match self {
            Operand::SmallConstant(v) => *v as i32,
            Operand::LargeConstant(v) => *v as i32,
            Operand::Variable(addr) => {
                let frame = state.current.get_mut();
                if *addr == 0 {
                    frame.stack.pop().expect("blown the stack") as i32
                } else if *addr >= 0x10 {
                    // global, not implemented yet
                    0
                } else {
                    frame.locals[*addr as usize] as i32
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct ZMachine {
    memory: ZMemory,
    state: ZMachineState,
}

#[derive(Default, Debug)]
struct Instruction {
    pub(crate) operands: Vec<Operand>,
    pub(crate) store: u8,
    pub(crate) jump: u16
}

impl Instruction {
    #[inline]
    pub fn new() -> Self {
        Instruction {
            operands: Vec::new(),
            store: 0,
            jump: 0
        }
    }

    #[inline]
    pub fn long(raw: u8, mem: &ZMemory, pc: &mut u16) -> Instruction {
        let mut instr = Instruction::new();
        if raw & 0x40 > 0 {
            instr.operands.push(Operand::Variable(mem.read_byte(*pc)));
        } else {
            instr.operands.push(Operand::SmallConstant(mem.read_byte(*pc)));
        }
        *pc += 1;

        if raw & 0x20 > 0 {
            instr.operands.push(Operand::Variable(mem.read_byte(*pc)));
        } else {
            instr.operands.push(Operand::SmallConstant(mem.read_byte(*pc)));
        }
        *pc += 1;

        instr
    }

    #[inline]
    pub fn long_store(raw: u8, mem: &ZMemory, pc: &mut u16) -> Instruction {
        let mut instr = Instruction::long(raw, mem, pc);
        instr.store = mem.read_byte(*pc);
        *pc += 1;

        instr
    }

    #[inline]
    pub fn variable(mem: &ZMemory, pc: &mut u16) -> Instruction {
        let mut result = Instruction::new();
        let mut types = mem.read_byte(*pc);
        *pc += 1;

        while types > 0 {
            let cur = types & 0xC0;
            match cur {
                0x00 => { // large constant
                    let val = mem.read_word(*pc);
                    *pc += 2;
                    result.operands.push(Operand::LargeConstant(val));
                },
                0x40 => { // small constant
                    let val = mem.read_byte(*pc);
                    *pc += 1;
                    result.operands.push(Operand::SmallConstant(val));
                },
                0x80 => { // variable
                    let val = mem.read_byte(*pc);
                    *pc += 1;
                    result.operands.push(Operand::Variable(val));
                },
                _ => break,
            };
            types <<= 2;
        };
        result
    }

    #[inline]
    pub fn variable_store(mem: &ZMemory, pc: &mut u16) -> Instruction {
        let mut result = Instruction::variable(mem, pc);
        result.store = mem.read_byte(*pc);
        *pc += 1;

        result
    }
}

impl ZMachine {
    pub fn new() -> ZMachine {
        ZMachine::default()
    }

    pub fn load(&mut self, filename: &str) -> std::io::Result<()> {
        let f = File::open(filename)?;
        let mut reader = BufReader::new(f);
        let mut buf: Vec<u8> = Vec::new();

        reader.read_to_end(&mut buf)?;
        self.memory.reset(buf);

        self.state.reset(&self.memory.header());

        println!("Version: {}", self.state.machine_version);
        println!("PC: {:x}", self.state.current.borrow().pc);
        Ok(())
    }

    pub fn exec_one(&mut self) -> bool {
        let frame = &mut self.state.current.get_mut();
        let opcode = self.memory.read_byte(frame.pc);
        frame.pc += 1;

        if opcode & 0xC0 == 0xC0 { // variable
            match opcode & 0x0F {
                0 => { // call
                    let instruction = Instruction::variable_store(&self.memory, &mut frame.pc);
                    println!("var instruction: {:x?}", instruction);

                    let routine_addr = instruction.operands[0].get_value(&mut self.state) as u16 * 2;
                    let mut locals: Vec<u16> = Vec::new();
                    let n_locals = self.memory.read_byte(routine_addr) as u16;

                    for i in 0..n_locals {
                        locals.push(self.memory.read_word(routine_addr + 1 + i * 2));
                    }

                    for (n, op) in instruction.operands[1..].iter().enumerate() {
                        locals[n] = op.get_value(&mut self.state) as u16
                    }

                    self.state.push_frame(StackFrame {
                        locals: locals,
                        stack: Vec::new(),
                        pc: routine_addr + 1 + n_locals * 2,
                        ret_addr: instruction.store,
                    });

                    return true;
                },
                code => println!("Unimplemented variable: raw {} opcode {}", opcode, code),
            }
        } else if opcode & 0xC0 == 0x80 { // short
            match opcode & 0x0F {
                code => println!("Unimplemented short: raw {} opcode {}", opcode, code),
            }

        } else { // long
            match opcode & 0x1F {
                3 => { // jump greater than
                    let instruction = Instruction::long_store(opcode, &self.memory, &mut frame.pc);
                    println!("jg: {:?}", instruction);
                }
                code => println!("Unimplemented long: raw {} opcode {}", opcode, code),
            }
        }

        false
    }

    fn store(&mut self, val: u8, addr: u8) {

    }
}
