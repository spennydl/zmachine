#![recursion_limit = "512"]

#[macro_use]
mod bits;

mod zmachine;
mod zmemory;
mod zinst;
mod zstr;
mod constants;

#[macro_use]
extern crate typenum;

pub use zmachine::{ZMachine};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
