use zmachine::{ZMachineExecResult, ZMachine};
use std::io;

fn main() {
    let mut machine = ZMachine::new();
    match machine.load("./games/zork1-r119-s880429.z3") {
        Ok(()) => {
            loop {
                match machine.exec() {
                    ZMachineExecResult::NEED_INPUT => {
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).expect("couldn't read from stdin");

                        machine.send_input(&input);
                    },
                    _ => break,
                }
            }
        },
        Err(e) => println!("whoops {}", e),
    };
}
