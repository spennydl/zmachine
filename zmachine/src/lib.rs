mod zmachine;
mod zmemory;

pub use zmachine::{ZMachine, ZMachineState};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
