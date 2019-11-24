use crate::zmemory::ZMemory;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::cell::RefCell;

use crate::zinst::{Instruction, InstructionType, Operand, Address};

#[derive(Default, Debug)]
pub struct StackFrame {
    locals: Vec<u16>,
    stack: Vec<u16>,
    ret_addr: u8,
    pc: u16,
}


#[derive(Default, Debug)]
pub struct ZMachine {
    memory: RefCell<ZMemory>,
    stack: RefCell<Vec<StackFrame>>,
}

impl ZMachine {
    pub fn new() -> ZMachine {
        ZMachine::default()
    }

    fn reset(&mut self, buf: Vec<u8>) {
        let mut mem = self.memory.borrow_mut();
        mem.reset(buf);

        let header = mem.header();
        self.stack.replace(vec![StackFrame::default()]);

        let mut stack = self.stack.borrow_mut();
        let current = &mut stack[0];
        let mut pc = header[6] as u16;
        pc = (pc << 8) | header[7] as u16;
        current.pc = pc;
    }

    pub fn load(&mut self, filename: &str) -> std::io::Result<()> {
        let f = File::open(filename)?;
        let mut reader = BufReader::new(f);
        let mut buf: Vec<u8> = Vec::new();
        reader.read_to_end(&mut buf)?;

        self.reset(buf);

        let current_frame = &self.stack.borrow()[0];
        println!("PC: {:x}", current_frame.pc);
        Ok(())
    }

    fn fetch_next_instr(&self) -> Instruction {
        let mut stack = self.stack.borrow_mut();
        let mem = self.memory.borrow();

        let idx = stack.len() - 1;
        let frame = &mut stack[idx];

        let (instr, offset) = Instruction::from_mem(mem.slice(frame.pc as usize));
        frame.pc += offset as u16;

        instr
    }

    pub fn exec_one(&self) -> bool {
        let instr = self.fetch_next_instr();

        let mut stack = self.stack.borrow_mut();
        let mut mem = self.memory.borrow_mut();
        // TODO: better stack
        let idx = stack.len() - 1;
        let mut current_frame = &mut stack[idx];

        match instr.ty {
            InstructionType::Long => {
                match instr.opcode {
                    1 => { // jump equal
                        println!("Jump equal: {:?}", instr);

                        let lhs = self.get_value(&instr.ops[0], &mut mem, &mut current_frame);
                        let rhs = self.get_value(&instr.ops[1], &mut mem, &mut current_frame);

                        println!("operands: {} {}", lhs, rhs);
                        self.branch_if(&mut mem, &mut current_frame.pc, lhs == rhs);
                        return true;
                    }
                    3 => { // jump greater than
                        println!("jg: {:?}", instr);
                    },
                    20 => { // add
                        let store = Address::of(mem.read_byte(current_frame.pc) as u16);
                        println!("add: {:?} {:?}", instr, store);
                        current_frame.pc += 1;
                        let lhs = self.get_value(&instr.ops[0], &mut mem, &mut current_frame);
                        let rhs = self.get_value(&instr.ops[1], &mut mem, &mut current_frame);

                        let result = lhs + rhs;
                        self.store(result, &store, &mut mem, &mut current_frame);

                        return true;
                    },
                    21 => { // sub
                        let store = Address::of(mem.read_byte(current_frame.pc) as u16);
                        println!("sub: {:?} {:?}", instr, store);
                        current_frame.pc += 1;
                        let lhs = self.get_value(&instr.ops[0], &mut mem, &mut current_frame);
                        let rhs = self.get_value(&instr.ops[1], &mut mem, &mut current_frame);

                        let result = lhs - rhs;
                        self.store(result, &store, &mut mem, &mut current_frame);

                        return true;
                    }
                    code => println!("Unimplemented long: raw {} opcode {}", instr.opcode, code),
                }

            },
            InstructionType::Short => {
                match instr.opcode {
                    _ => println!("Unimplemented short: {:?}", instr),
                }
            },
            InstructionType::Variable => {
                match instr.opcode {
                    0 => { // call
                        println!("var instruction: {:x?}", instr);
                        let store = mem.read_byte(current_frame.pc);
                        current_frame.pc += 1;

                        let routine_addr = instr.ops[0].value() * 2;
                        let mut locals: Vec<u16> = Vec::new();
                        let n_locals = mem.read_byte(routine_addr) as u16;

                        for i in 0..n_locals {
                            locals.push(mem.read_word(routine_addr + 1 + i * 2));
                        }

                        for (n, op) in instr.ops[1..].iter().enumerate() {
                            locals[n] = op.value();
                        }

                        stack.push(StackFrame {
                            locals,
                            stack: Vec::new(),
                            pc: routine_addr + 1 + n_locals * 2,
                            ret_addr: store,
                        });

                        return true;
                    },
                    code => println!("Unimplemented variable: raw {} opcode {}", instr.opcode, code),
                }
            }
        }

        false
    }

    fn branch_if(&self, mem: &mut ZMemory, pc: &mut u16, cond: bool) {
        let branch_hi = mem.read_byte(*pc);
        *pc += 1;

        let inv = branch_hi & 0x80 > 0;
        let mut offset: i32 = 0;
        if branch_hi & 0x40 > 0 {
            offset = (branch_hi & 0x3F) as i32;
        } else {
            let neg = branch_hi & 0x20 > 0;
            let val = branch_hi & 0x1F;
            let low = mem.read_byte(*pc);
            *pc += 1;
            offset = ((val as u16) << 8 | low as u16) as i32;
            if neg {
                offset = offset * -1;
            }
        }

        if (cond && !inv) || (!cond && inv) {
            *pc = ((*pc as i32) + offset as i32) as u16 - 2; // minus 2 for some reason?
        }
    }

    fn store(&self, val: u16, addr: &Address, mem: &mut ZMemory, frame: &mut StackFrame) {
        match addr {
            Address::Global(a) => {
                mem.set_global(*a as u8, val);
            },
            Address::StackPointer => {
                frame.stack.push(val);
            },
            Address::Local(a) => {
                frame.locals[*a as usize] = val;
            }
        }
    }

    fn get_value(&self, v: &Operand, mem: &mut ZMemory, frame: &mut StackFrame) -> u16 {
        match v {
            Operand::Variable(a) => {
                match a {
                    Address::Global(addr) => {
                        mem.global(*addr as u8) as u16
                    },
                    Address::StackPointer => {
                        frame.stack.pop().expect("blew the stack")
                    },
                    Address::Local(addr) => {
                        frame.locals[*addr as usize]
                    }
                }
            },
            _ => v.value(),
        }
    }
}
