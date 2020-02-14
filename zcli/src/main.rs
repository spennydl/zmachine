use zmachine::{ZMachineExecResult, ZMachine};

use std::io;
use std::io::Write;
use std::env;

fn main() {
    let mut machine = ZMachine::new();
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("no args");
    } else {
        let mut game = String::from("./games/");
        game.push_str(&args[1]);

        let mut output: Vec<u8> = Vec::new();

        match machine.load(&game) {
            Ok(()) => {
                loop {
                    match machine.exec(&mut output) {
                        ZMachineExecResult::NeedInput => {
                            io::stdout().write_all(&output[..]).unwrap();
                            io::stdout().flush().unwrap();
                            output = Vec::new();

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
}
