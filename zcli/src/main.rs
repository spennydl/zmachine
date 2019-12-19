use zmachine::{ZMachineExecResult, ZMachine};
use std::io;

fn main() {
    let mut machine = ZMachine::new();
    match machine.load("./games/hitchhiker-r60-s861002.z3") {
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
