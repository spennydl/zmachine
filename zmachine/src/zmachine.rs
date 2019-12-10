use crate::zmemory::ZMemory;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::cell::RefCell;
use std::convert::TryInto;

use crate::zinst::{Instruction, InstructionType, Operand, Address, BranchLabel, Offset};
use crate::bits::ZWord;
use crate::zstr::ZCharWord;

#[derive(Debug)]
struct BranchOffset {
    target: bool,
    offset: Offset
}

impl BranchOffset {
    fn new(target: bool, offset: Offset) -> BranchOffset {
        BranchOffset { target, offset }
    }

    fn target(&self) -> &bool {
        &self.target
    }

    fn offset(&self) -> &Offset {
        &self.offset
    }
}

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
    input_buffer: RefCell<Option<(u16, u16)>>,
}

pub enum ZMachineExecResult {
    NEED_INPUT,
    NEXT,
    EXIT
}

struct ZLexicalAnalyzer<'a> {
    mem: &'a mut ZMemory,
    tb_addr: u16,
    pb_addr: u16,
}

struct ZDictEntry {
    chars: (ZCharWord, ZCharWord)
}

impl ZDictEntry {

    fn as_u32(&self) -> u32 {
        let (ref hi, ref low) = self.chars;
        let hi = hi.get().to_be_bytes();
        let lo = low.get().to_be_bytes();
        let slice = &[hi, lo].concat();

        u32::from_be_bytes(slice.as_slice().try_into().unwrap())
    }

    fn from_slice(slice: &[u8]) -> ZDictEntry {
        let mut lo = ZCharWord::new(0);
        let mut hi = ZCharWord::new(0);

        if let Some(c) = slice.get(0) {
            hi.first.set(*c as u16);
        } else {
            hi.last_flag.set(1);
            hi.first.set(5);
        }
        
        if let Some(c) = slice.get(1) {
            hi.second.set(*c as u16);
        } else {
            hi.last_flag.set(1);
            hi.second.set(5);
        }
        
        if let Some(c) = slice.get(2) {
            hi.third.set(*c as u16);
        } else {
            hi.last_flag.set(1);
            hi.third.set(5);
        }

        if let Some(c) = slice.get(3) {
            lo.first.set(*c as u16);
        } else {
            lo.first.set(5);
        }

        if let Some(c) = slice.get(4) {
            lo.second.set(*c as u16);
        } else {
            lo.second.set(5);
        }

        if let Some(c) = slice.get(5) {
            lo.third.set(*c as u16);
        } else {
            lo.third.set(5);
        }

        lo.last_flag.set(1);

        ZDictEntry { chars: (hi, lo) }
    }
}

impl<'a> ZLexicalAnalyzer<'a> {
    pub fn new(mem: &'a mut ZMemory, tb_addr: u16, pb_addr: u16) -> ZLexicalAnalyzer<'a> {
        ZLexicalAnalyzer { mem, tb_addr, pb_addr }
    }

    fn run(&self) {
        let text_addr = self.tb_addr as usize + 1;
        let pb_addr = self.pb_addr as usize + 1;

        let dictionary = self.mem.dictionary();
        let separators = dictionary.separators();

        let text = self.mem.slice(text_addr);
        let mut words: Vec<&[u8]> = Vec::new();

        let mut idx: usize = 0;
        for (i, ch) in text.iter()
            .take_while(|c| **c != 0)
            .enumerate() {
            if separators.contains(ch) {
                let word = &text[idx..i];
                let sep_word = &text[i..i + 1];
                words.push(word);
                words.push(sep_word);
                idx = i + 1;
            } else if *ch == ' ' as u8 {
                let word = &text[idx..i]; // don't include the space
                idx = i + 1;
                words.push(word);
            }
        }

        for (word, dict_entry) in words.iter().map(|w| (w, ZDictEntry::from_slice(w).as_u32())) {
            if dictionary.contains(&dict_entry) {

            }
        }

    }
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

        //let current_frame = &self.stack.borrow()[0];
        //println!("PC: {:x}", current_frame.pc);
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

    fn store_text_buffer(&self, tb: u16, pb: u16) {
        let mut buffer = self.input_buffer.borrow_mut();
        buffer.replace((tb, pb));
    }

    fn get_pc(&self) -> usize {
        let stack = self.stack.borrow();
        let idx = stack.len() - 1;
        
        stack[idx].pc
    }

    fn read_store(&self, pc: &mut usize) -> Address {
        let mem = self.memory.borrow();
        let addr = Address::of(mem.read_byte(*pc) as u16);
        *pc += 1;

        addr
    }

    pub fn send_input(&self, input: &str) {
        println!("received {}", input);
        if let Some((text_buffer_addr, parse_buffer_addr)) = *self.input_buffer.borrow() {
            let mut mem = self.memory.borrow_mut();
            mem.write_text(text_buffer_addr, &input[..]);
            
            let analyzer = ZLexicalAnalyzer::new(&mut mem, text_buffer_addr, parse_buffer_addr);
            analyzer.run();
        }

        self.input_buffer.replace(None);
    }

    pub fn exec(&self) -> ZMachineExecResult {
        loop {
            match self.exec_one() {
                ZMachineExecResult::NEXT => continue,
                result => return result
            }
        }
    }

    fn exec_one(&self) -> ZMachineExecResult {
        let instr = self.fetch_next_instr();

        let mut pc = self.get_pc();
        //println!("PC is {:x}", pc);

        match instr.ty {
            InstructionType::Long => {
                match instr.opcode {
                    1 => { // jump equal
                        //println!("Jump equal: {:?}", instr);

                        let lhs = self.get_value(&instr.ops[0]) as i16;
                        let rhs = self.get_value(&instr.ops[1]) as i16;
                        let cond = lhs == rhs;

                        let offset = self.read_offset(&mut pc);
                        if cond == *offset.target() {
                            self.branch(offset, &mut pc);
                        }
                    }
                    3 => { // jump greater than
                        //println!("je: {:?}", instr);
                        let lhs = self.get_value(&instr.ops[0]) as i16;
                        let rhs = self.get_value(&instr.ops[1]) as i16;
                        let cond = lhs > rhs;

                        let offset = self.read_offset(&mut pc);
                        if cond == *offset.target() {
                            self.branch(offset, &mut pc);
                        }
                    },
                    5 => { // inc check
                        //println!("inc check: {:?}", instr);

                        let var_num = self.get_value(&instr.ops[0]);
                        let var_addr = Address::of(var_num);
                        let op = Operand::Variable(var_addr);

                        let mut var = self.get_value(&op) as i16;
                        let val = self.get_value(&instr.ops[1]) as i16;

                        var += 1;
                        let cond = var > val;

                        let addr = Address::of(var_num);
                        self.store(var as u16, &addr);

                        let offset = self.read_offset(&mut pc);
                        if cond == offset.target {
                            self.branch(offset, &mut pc);
                        }
                    },
                    7 => { // test
                        //println!("test {:?}", instr);
                        let bm = self.get_value(&instr.ops[0]);
                        let flags = self.get_value(&instr.ops[0]);
                        let cond = bm & flags == flags;

                        let offset = self.read_offset(&mut pc);
                        if cond == offset.target {
                            self.branch(offset, &mut pc);
                        }
                    },
                    10 => { // test_attr
                        //println!("test attr {:?}", instr);
                        let obj_num = self.get_value(&instr.ops[0]);
                        let attr = self.get_value(&instr.ops[1]);
                        let offset = self.read_offset(&mut pc);

                        let mem = self.memory.borrow();
                        let attr = mem.test_attr(obj_num as u8, attr as u8);

                        if attr == offset.target {
                            self.branch(offset, &mut pc);
                        }
                    },
                    13 => { // store
                        //println!("store {:?}", instr);
                        let addr = Address::of(instr.ops[0].value());
                        let val = self.get_value(&instr.ops[1]);

                        self.store(val, &addr);
                    },
                    14 => { // insert_obj
                        //println!("insert_obj {:?}", instr);
                        let obj_num = self.get_value(&instr.ops[0]) as u8;
                        let new_parent = self.get_value(&instr.ops[1]) as u8;

                        let mut mem = self.memory.borrow_mut();
                        mem.insert_object(obj_num, new_parent);
                    }
                    15 => { // loadw
                        //println!("loadw {:?}", instr);
                        let store = self.read_store(&mut pc);

                        let addr = self.get_value(&instr.ops[0]);
                        let idx = self.get_value(&instr.ops[1]);
                        let addr = Address::Word(addr + (2 * idx));

                        let val = self.get_value(&Operand::Variable(addr));
                        self.store(val, &store);
                    },
                    16 => { // loadb
                        //println!("loadb {:?}", instr);
                        let addr = self.get_value(&instr.ops[0]);
                        let idx = self.get_value(&instr.ops[1]);
                        let store = self.read_store(&mut pc);
                        let addr = Address::Byte(addr + idx);

                        let val = self.get_value(&Operand::Variable(addr));
                        let val = (val >> 8) as u8;
                        self.store(val as u16, &store);
                    },
                    18 => { // mod a b
                        //println!("mod: {:?}", instr);
                        let lhs = self.get_value(&instr.ops[0]) as i16;
                        let rhs = self.get_value(&instr.ops[1]) as i16;
                        let store = self.read_store(&mut pc);

                        let result = lhs % rhs;
                        self.store(result as u16, &store);
                    },
                    20 => { // add
                        let lhs = self.get_value(&instr.ops[0]) as i16;
                        let rhs = self.get_value(&instr.ops[1]) as i16;
                        let store = self.read_store(&mut pc);
                        //println!("add: {:?}", instr);

                        let result = lhs + rhs;
                        self.store(result as u16, &store);
                    },
                    21 => { // sub
                        let lhs = self.get_value(&instr.ops[0]) as i16;
                        let rhs = self.get_value(&instr.ops[1]) as i16;
                        let store = self.read_store(&mut pc);
                        //println!("sub: {:?}", instr);

                        let result = lhs - rhs;
                        self.store(result as u16, &store);
                    }
                    _code => {
                        println!("Unimplemented long: {:?} pc {:x}", instr, pc);
                        return ZMachineExecResult::EXIT;
                    }
                }

            },
            InstructionType::ZeroOps => {
                match instr.opcode {
                    0 => { // return true
                        //println!("return true {:?}", instr);
                        pc = self.return_val(1);
                    },
                    1 => { // return false
                        //println!("return false {:?}", instr);
                        pc = self.return_val(0);
                    },
                    2 => { // PRINT!!!!
                        //println!("print: {:?}", instr);

                        let mem = self.memory.borrow();
                        let (message, offset) = mem.read_string(pc as u16);
                        pc += offset;

                        print!("{}", message);
                    },
                    8 => { // ret popped
                        //println!("ret popped {:?}", instr);
                        let val = self.get_value(&Operand::Variable(Address::StackPointer));
                        pc = self.return_val(val);
                    },
                    11 => {
                        //println!("newline: {:?} pc: {:x}", instr, pc);
                        
                        println!();
                    },
                    _code => {
                        println!("unimplemented no-op: {:?} pc: {:x}", instr, pc);
                        return ZMachineExecResult::NEXT;
                    }
                }
            },
            InstructionType::Short => {
                match instr.opcode {
                    0 => { // jz
                        //println!("Jump zero: {:?}", instr);

                        let val = self.get_value(&instr.ops[0]) as i16;
                        let offset = self.read_offset(&mut pc);
                        let cond = val == 0;

                        //println!("operands: {}", val);
                        if cond == offset.target {
                            self.branch(offset, &mut pc);
                        }
                    },
                    1 => { // get sibling
                        //println!("get sibling {:?}", instr);
                        let store = self.read_store(&mut pc);
                        let offset = self.read_offset(&mut pc);
                        let obj_num = self.get_value(&instr.ops[0]) as u8;

                        let num = {
                            let mem = self.memory.borrow();
                            mem.get_object_sibling(obj_num)
                        };

                        if let Some(sib) = num {
                            self.store(sib as u16, &store);
                            self.branch(offset, &mut pc);
                        } else {
                            self.store(0, &store); // think i still need to do this?
                        }
                    },
                    2 => { // get child
                        //println!("get sibling {:?}", instr);
                        let store = self.read_store(&mut pc);
                        let offset = self.read_offset(&mut pc);
                        let obj_num = self.get_value(&instr.ops[0]) as u8;

                        let num = {
                            let mem = self.memory.borrow();
                            mem.get_object_child(obj_num)
                        };

                        if let Some(sib) = num {
                            self.store(sib as u16, &store);
                            self.branch(offset, &mut pc);
                        } else {
                            self.store(0, &store); // think i still need to do this?
                        }
                    }
                    3 => { // get parent
                        //println!("get parent {:?}", instr);
                        let store = self.read_store(&mut pc);
                        let obj_num = self.get_value(&instr.ops[0]) as u8;

                        let num = {
                            let mem = self.memory.borrow();
                            mem.get_object_parent(obj_num)
                        };

                        self.store(num as u16, &store);
                    },
                    11 => { // return value
                        //println!("return a val {:?}", instr);
                        let val = self.get_value(&instr.ops[0]);
                        pc = self.return_val(val);
                    },
                    12 => { // jump
                        //println!("jump {:?}", instr);
                        let jmp = self.get_value(&instr.ops[0]) as i16;
                        let offset = BranchOffset::new(true, Offset::Signed(jmp));
                        self.branch(offset, &mut pc);
                    },
                    _ => {
                        println!("Unimplemented short: {:?} pc {:x}", instr, pc);
                        return ZMachineExecResult::EXIT;
                    }
                }
            },
            InstructionType::Variable => {
                match instr.opcode {
                    0 => { // call
                        //println!("call: {:x?}", instr);
                        let store = self.read_store(&mut pc);
                        let mem = self.memory.borrow();

                        let routine_addr = instr.ops[0].value() as usize * 2;
                        let n_locals = mem.read_byte(routine_addr) as usize;

                        let mut locals: Vec<u16> = Vec::new();
                        for i in 0..n_locals {
                            locals.push(mem.read_word(routine_addr + 1 + i * 2).into());
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
                        return ZMachineExecResult::NEXT;
                    },
                    1 => { // storew
                        //println!("storew {:?}", instr);
                        let addr = self.get_value(&instr.ops[0]);
                        let idx = self.get_value(&instr.ops[1]);
                        let val = self.get_value(&instr.ops[2]);
                        let addr = Address::Word(addr + (2 * idx));

                        self.store(val, &addr);
                    },
                    2 => { // storeb
                        let addr = self.get_value(&instr.ops[0]);
                        let idx = self.get_value(&instr.ops[1]);
                        let val = self.get_value(&instr.ops[2]);
                        let addr = Address::Byte(addr + idx);

                        self.store(val, &addr);
                    }
                    3 => { // put prop 
                        //println!("put prop: instr {:?} pc {:x}", instr, pc);
                        let obj_num = self.get_value(&instr.ops[0]);
                        let prop_num = self.get_value(&instr.ops[1]) as u8;
                        let val = self.get_value(&instr.ops[2]);
                        
                        let mut mem = self.memory.borrow_mut();
                        mem.put_prop(obj_num as u8, prop_num, val.into());
                    },
                    4 => { // read!
                        let text_buffer_addr = self.get_value(&instr.ops[0]);
                        let parse_buffer_addr = self.get_value(&instr.ops[1]);

                        self.store_text_buffer(text_buffer_addr, parse_buffer_addr);

                        return ZMachineExecResult::NEED_INPUT;
                    },
                    5 => { // print char
                        //println!("print char: instr {:?} pc {:x}", instr, pc);
                        let ch = self.get_value(&instr.ops[0]) as u8;
                        print!("{}", ch as char);
                    },
                    6 => { // print num
                        //println!("print num: {:?}", instr);
                        let val = self.get_value(&instr.ops[0]) as i16;
                        print!("{}", val as i16);
                    },
                    9 => { // AND
                        //println!("and {:?}", instr);
                        let store = self.read_store(&mut pc);
                        let lhs = self.get_value(&instr.ops[0]);
                        let rhs = self.get_value(&instr.ops[1]);

                        let result = lhs & rhs;

                        self.store(result, &store);
                    },
                    13 => { // var store
                        //println!("var store {:?}", instr);
                        let addr = Address::of(instr.ops[0].value());
                        let val = self.get_value(&instr.ops[1]);

                        self.store(val, &addr);
                    },
                    _code => {
                        println!("Unimplemented variable: instr {:?} pc {:x}", instr, pc);
                        return ZMachineExecResult::EXIT;
                    }
                }
            }
        }

        let mut stack = self.stack.borrow_mut();
        let idx = stack.len() - 1;
        let current_frame = &mut stack[idx];

        current_frame.pc = pc;

        ZMachineExecResult::NEXT
    }

    fn read_offset(&self, pc: &mut usize) -> BranchOffset {
        let mem = self.memory.borrow();
        
        let val = mem.read_word(*pc).into();
        let branch_label = BranchLabel::new(val);

        let target = branch_label.invert.is_set();

        let offset = if branch_label.offset.is_set() {
            *pc += 1; // branch was only a byte
            branch_label.unsigned_value.value_of() as i16
        } else {
            *pc += 2; // branch was 2 bytes
            if branch_label.sign.is_set() {
                (16383 - branch_label.signed_value.value_of()) as i16 * -1
            } else {
                branch_label.signed_value.value_of() as i16
            }
        };

        let offset = if offset == 0 {
            Offset::RFalse
        } else if offset == 1 {
            Offset::RTrue
        } else if branch_label.offset.is_set() {
            Offset::Unsigned(offset as u8)
        } else {
            Offset::Signed(offset)
        };

        BranchOffset::new(target, offset)
    }

    fn return_val(&self, val: u16) -> usize {
        let old_frame = {
            let mut stack = self.stack.borrow_mut();
            stack.pop().expect("blew the stack!")
        };
        self.store(val, &old_frame.ret_addr);

        let stack = self.stack.borrow();
        let idx = stack.len() - 1;
        
        stack[idx].pc
    }

    fn branch(&self, offset: BranchOffset, pc: &mut usize) {
        match offset.offset() {
            Offset::RFalse => {
                let new_pc = self.return_val(0);
                *pc = new_pc;
            },
            Offset::RTrue => {
                let new_pc = self.return_val(1);
                *pc = new_pc;
            },
            Offset::Unsigned(off) => {
                let new_pc = (*pc + *off as usize) - 2;
                *pc = new_pc;
            },
            Offset::Signed(mut off) => {
                if off < 0 {
                    off = off * -1;
                    let new_pc = (*pc - off as usize) - 2;
                    *pc = new_pc;
                } else {
                    let new_pc = (*pc + off as usize) - 2;
                    *pc = new_pc;
                }
            }
        }
    }

    fn store(&self, val: u16, addr: &Address) {
        let mut mem = self.memory.borrow_mut();
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
            Address::Byte(a) => {
                mem.set_byte(*a as usize, val as u8);
            }
        }
    }

    fn get_value(&self, v: &Operand) -> u16 {
        match v {
            Operand::Variable(a) => {
                match a {
                    Address::Global(addr) => {
                        let mem = self.memory.borrow();
                        mem.global(*addr as usize).into()
                    },
                    Address::StackPointer => {
                        let mut stack = self.stack.borrow_mut();
                        let idx = stack.len() - 1;
                        let frame = &mut stack[idx];
                        frame.stack.pop().expect("blew the stack")
                    },
                    Address::Local(addr) => {
                        let mut stack = self.stack.borrow_mut();
                        let idx = stack.len() - 1;
                        let frame = &mut stack[idx];
                        frame.locals[*addr as usize]
                    },
                    Address::Word(addr) => {
                        let mem = self.memory.borrow();
                        mem.read_word(*addr as usize).into()
                    },
                    Address::Byte(addr) => {
                        let mem = self.memory.borrow();
                        mem.read_byte(*addr as usize).into()
                    }
                }
            },
            _ => v.value(),
        }
    }
}
