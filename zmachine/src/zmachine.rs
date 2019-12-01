use crate::zmemory::ZMemory;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::cell::RefCell;

use crate::zinst::{Instruction, InstructionType, Operand, Address, BranchLabel};
use crate::bits::ZWord;

#[derive(Default, Debug)]
pub struct StackFrame {
    locals: Vec<u16>,
    stack: Vec<u16>,
    ret_addr: Address,
    pc: usize,
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

        let pc: ZWord = (header[6], header[7]).into();
        current.pc = u16::from(pc) as usize;
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

        let (instr, offset) = Instruction::from_mem(mem.slice(frame.pc));
        frame.pc += offset;

        instr
    }

    fn get_pc(&self) -> usize {
        let stack = self.stack.borrow();
        let idx = stack.len() - 1;
        
        stack[idx].pc
    }

    pub fn exec_one(&self) -> bool {
        let instr = self.fetch_next_instr();

        let mut pc = self.get_pc();
        let mut mem = self.memory.borrow_mut();
        println!("PC is {:x}", pc);

        match instr.ty {
            InstructionType::Long => {
                match instr.opcode {
                    1 => { // jump equal
                        println!("Jump equal: {:?}", instr);

                        let lhs = self.get_value(&instr.ops[0], &mem) as i16;
                        let rhs = self.get_value(&instr.ops[1], &mem) as i16;

                        println!("operands: {} {}", lhs, rhs);
                        self.branch_if(&mem, &mut pc, lhs == rhs);
                    }
                    3 => { // jump greater than
                        println!("jg: {:?}", instr);
                        println!("unimplemented!");
                    },
                    5 => { // inc check
                        println!("inc check: {:?}", instr);
                        let var_num = self.get_value(&instr.ops[0], &mem);
                        let var_addr = Address::of(var_num);
                        let op = Operand::Variable(var_addr);
                        let mut var = self.get_value(&op, &mem);
                        let val = self.get_value(&instr.ops[1], &mem);

                        var += 1;

                        let addr = Address::of(var_num);
                        self.store(var, &addr, &mut mem);
                        println!("comparing {} and {}", var, val);
                        self.branch_if(&mem, &mut pc, var as i16 > val as i16);

                    },
                    10 => { // test_attr
                        println!("test attr {:?}", instr);
                        let obj_num = self.get_value(&instr.ops[0], &mem);
                        let attr = self.get_value(&instr.ops[1], &mem);

                        let attr = mem.test_attr(obj_num as u8, attr as u8);
                        self.branch_if(&mem, &mut pc, attr);
                    },
                    13 => { // store
                        println!("store {:?}", instr);
                        let addr = Address::of(self.get_value(&instr.ops[0], &mem));
                        let val = self.get_value(&instr.ops[1], &mem);

                        self.store(val, &addr, &mut mem);
                    },
                    14 => { // insert_obj
                        println!("insert_obj {:?}", instr);
                        let obj_num = self.get_value(&instr.ops[0], &mem) as u8;
                        let new_parent = self.get_value(&instr.ops[1], &mem) as u8;

                        mem.insert_object(obj_num, new_parent);
                    }
                    15 => { // loadw
                        println!("loadw {:?}", instr);
                        let store = Address::of(mem.read_byte(pc) as u16);
                        pc += 1;
                        let addr = self.get_value(&instr.ops[0], &mem);
                        let idx = self.get_value(&instr.ops[1], &mem);
                        let addr = Address::Word(addr + (2 * idx));


                        println!("loading from {:?} to {:?}", addr, store);
                        let val = self.get_value(&Operand::Variable(addr), &mem);
                        self.store(val, &store, &mut mem);
                    },
                    16 => { // loadb
                        println!("loadb {:?}", instr);
                        let store = Address::of(mem.read_byte(pc) as u16);
                        pc += 1;
                        let addr = self.get_value(&instr.ops[0], &mem);
                        let idx = self.get_value(&instr.ops[1], &mem);
                        let addr = Address::Word(addr + (idx));


                        println!("loading byte from {:?} to {:?}", addr, store);
                        let val = self.get_value(&Operand::Variable(addr), &mem);
                        let val = (val >> 8) as u8;
                        self.store(val as u16, &store, &mut mem);
                    },
                    18 => { // mod a b
                        println!("mod: {:?}", instr);
                        println!("unimplemented!");
                        return false;
                    },
                    20 => { // add
                        let store = Address::of(mem.read_byte(pc) as u16);
                        println!("add: {:?} {:?}", instr, store);
                        pc += 1;
                        let lhs = self.get_value(&instr.ops[0], &mem) as i16;
                        let rhs = self.get_value(&instr.ops[1], &mem) as i16;

                        let result = lhs + rhs;
                        self.store(result as u16, &store, &mut mem);
                    },
                    21 => { // sub
                        let store = Address::of(mem.read_byte(pc) as u16);
                        println!("sub: {:?} {:?}", instr, store);
                        pc += 1;
                        let lhs = self.get_value(&instr.ops[0], &mem) as i16;
                        let rhs = self.get_value(&instr.ops[1], &mem) as i16;

                        let result = lhs - rhs;
                        self.store(result as u16, &store, &mut mem);
                    }
                    _code => {
                        println!("Unimplemented long: {:?} pc {:x}", instr, pc);
                        return false;
                    }
                }

            },
            InstructionType::ZeroOps => {
                match instr.opcode {
                    0 => { // return true
                        println!("return true {:?}", instr);
                        // TODO: refactor return logic into a function
                        /*
                        println!("return true {:?}", instr);
                        let ret_val = 1;
                        let current_frame = stack.pop().expect("blown the stack");
                        let idx = stack.len() - 1;
                        let mut next_frame = &mut stack[idx];
                        
                        self.store(ret_val, &current_frame.ret_addr, &mut mem);
                        return true;
                        */
                        println!("unimplemented!");
                        return false;
                    },
                    1 => { // return false
                        println!("return false {:?}", instr);
                        // TODO: refactor return logic into a function
                        /*
                        let ret_val = 0;
                        let current_frame = stack.pop().expect("blown the stack");
                        let idx = stack.len() - 1;
                        let mut next_frame = &mut stack[idx];
                        
                        self.store(ret_val, &current_frame.ret_addr, &mut mem, &mut next_frame);
                        return true;
                        */
                        println!("unimplemented!");
                        return false;
                    },
                    2 => { // PRINT!!!!
                        println!("print: {:?}", instr);

                        let (message, offset) = mem.read_string(pc as u16);
                        pc += offset;

                        for line in message.lines() {
                            println!("ZZZZZ  {}", line);
                        }
                    },
                    11 => {
                        println!("newline: {:?} pc: {:x}", instr, pc);
                        
                        println!("{}", "ZZZZZ");
                    },
                    _code => {
                        println!("unimplemented no-op: {:?} pc: {:x}", instr, pc);
                        return false;
                    }
                }
            },
            InstructionType::Short => {
                match instr.opcode {
                    0 => { // jz
                        println!("Jump zero: {:?}", instr);

                        let val = self.get_value(&instr.ops[0], &mem) as i16;

                        println!("operands: {}", val);
                        self.branch_if(&mem, &mut pc, val == 0);
                    },
                    11 => { // return value
                        println!("return a val {:?}", instr);
                        /*
                        let ret_val = self.get_value(&instr.ops[0], &mem) as u16;
                        let current_frame = stack.pop().expect("blown the stack");
                        let idx = stack.len() - 1;
                        let mut next_frame = &mut stack[idx];
                        
                        self.store(ret_val, &current_frame.ret_addr, &mut mem, &mut next_frame);
                        return true;
                        */
                        println!("unimplemented!");
                        return false;
                    },
                    12 => { // jump
                        println!("jump {:?}", instr);
                        let mut jmp = self.get_value(&instr.ops[0], &mem) as i16;
                        if jmp < 0 {
                            jmp = jmp * -1;
                            pc = pc - jmp as usize - 2;
                        } else {
                            pc = pc + jmp as usize - 2;
                        }
                    },
                    _ => {
                        println!("Unimplemented short: {:?} pc {:x}", instr, pc);
                        return false;
                    }
                }
            },
            InstructionType::Variable => {
                match instr.opcode {
                    0 => { // call
                        println!("call: {:x?}", instr);
                        let store = Address::of(mem.read_byte(pc) as u16);
                        pc += 1;

                        let routine_addr = instr.ops[0].value() as usize * 2;
                        let mut locals: Vec<u16> = Vec::new();
                        let n_locals = mem.read_byte(routine_addr) as u16;

                        for i in 0..n_locals {
                            locals.push(mem.read_word(routine_addr as usize + 1 + i as usize * 2).into());
                        }

                        for (n, op) in instr.ops[1..].iter().enumerate() {
                            locals[n] = op.value();
                        }

                        let mut stack = self.stack.borrow_mut();
                        let idx = stack.len() - 1;
                        let current_frame = &mut stack[idx];
                        current_frame.pc = pc;

                        stack.push(StackFrame {
                            locals,
                            stack: Vec::new(),
                            pc: routine_addr as usize + 1 + n_locals as usize * 2,
                            ret_addr: store,
                        });

                        // return true here, we've already updated the pc
                        return true;
                    },
                    1 => { // storew
                        println!("storew {:?}", instr);
                        let addr = self.get_value(&instr.ops[0], &mem);
                        let idx = self.get_value(&instr.ops[1], &mem);
                        let addr = Address::Word(addr + (2 * idx));
                        let val = self.get_value(&instr.ops[2], &mem);

                        self.store(val, &addr, &mut mem);
                    },
                    3 => { // put prop - objects eek!
                        println!("put prop: instr {:?} pc {:x}", instr, pc);
                        let obj_num = self.get_value(&instr.ops[0], &mem);
                        let prop_num = self.get_value(&instr.ops[1], &mem) as u8;
                        let val = self.get_value(&instr.ops[2], &mem);
                        
                        mem.put_prop(obj_num as u8, prop_num, val.into());
                    },
                    5 => { // print char
                        println!("print char: instr {:?} pc {:x}", instr, pc);
                        let ch = self.get_value(&instr.ops[0], &mem) as u8;
                        println!("ZZZZZ {}", ch as char);
                    },
                    6 => { // print num
                        println!("print num: {:?}", instr);
                        let val = self.get_value(&instr.ops[0], &mem) as i16;
                        println!("ZZZZZ {}", val as i16);
                    },
                    9 => { // AND
                        println!("and {:?}", instr);
                        let store = Address::of(mem.read_byte(pc) as u16);
                        pc += 1;
                        let lhs = self.get_value(&instr.ops[0], &mem);
                        let rhs = self.get_value(&instr.ops[1], &mem);

                        let result = lhs & rhs;

                        self.store(result, &store, &mut mem);
                    },
                    _code => {
                        println!("Unimplemented variable: instr {:?} pc {:x}", instr, pc);
                    }
                }
            }
        }

        let mut stack = self.stack.borrow_mut();
        let idx = stack.len() - 1;
        let current_frame = &mut stack[idx];

        current_frame.pc = pc;

        true
    }

    fn branch_if(&self, mem: &ZMemory, pc: &mut usize, cond: bool) {
        let val = mem.read_word(*pc).into();
        let branch_label = BranchLabel::new(val);

        let target = branch_label.invert.is_set();

        let offset = if branch_label.offset.is_set() {
            *pc += 1; // branch was only a byte
            branch_label.unsigned_value.value_of() as i16
        } else {
            *pc += 2; // branch was 2 bytes
            if branch_label.sign.is_set() {
                (!branch_label.signed_value.value_of() + 1) as i16 * -1
            } else {
                branch_label.signed_value.value_of() as i16
            }
        };

        println!("offset is {}", offset);
        if cond == target {
            let new_pc = ((*pc as i16) + offset) as usize - 2;
            println!("branching, pc is now {:x}", new_pc);
            *pc = new_pc;
        }
    }

    fn store(&self, val: u16, addr: &Address, mem: &mut ZMemory) {
        let mut stack = self.stack.borrow_mut();
        let idx = stack.len() - 1;
        let frame = &mut stack[idx];
        match addr {
            Address::Global(a) => {
                mem.set_global(*a as usize, val.into());
            },
            Address::StackPointer => {
                frame.stack.push(val);
            },
            Address::Local(a) => {
                frame.locals[*a as usize] = val;
            },
            Address::Word(a) => {
                mem.set_word(*a as usize, val.into());
            },
        }
    }

    fn get_value(&self, v: &Operand, mem: &ZMemory) -> u16 {
        let mut stack = self.stack.borrow_mut();
        let idx = stack.len() - 1;
        let frame = &mut stack[idx];
        match v {
            Operand::Variable(a) => {
                match a {
                    Address::Global(addr) => {
                        mem.global(*addr as usize).into()
                    },
                    Address::StackPointer => {
                        frame.stack.pop().expect("blew the stack")
                    },
                    Address::Local(addr) => {
                        frame.locals[*addr as usize]
                    },
                    Address::Word(addr) => {
                        mem.read_word(*addr as usize).into()
                    },
                }
            },
            _ => v.value(),
        }
    }
}
