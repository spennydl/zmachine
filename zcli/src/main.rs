use zmachine::ZMachine;

fn main() {
    let mut machine = ZMachine::new();
    match machine.load("./games/zork1-r119-s880429.z3") {
        Ok(()) => {
            while machine.exec_one() {}
        },
        Err(e) => println!("whoops {}", e),
    };
}
