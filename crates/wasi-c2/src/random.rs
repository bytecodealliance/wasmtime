use crate::Error;
use std::cell::RefCell;

pub trait WasiRandom {
    fn get(&self, buf: &mut [u8]) -> Result<(), Error>;
}

pub struct GetRandom;

impl WasiRandom for GetRandom {
    fn get(&self, buf: &mut [u8]) -> Result<(), Error> {
        getrandom::getrandom(buf)?;
        Ok(())
    }
}

pub struct Deterministic {
    sequence: RefCell<std::iter::Cycle<std::vec::IntoIter<u8>>>,
}

impl Deterministic {
    pub fn new(bytes: Vec<u8>) -> Self {
        Deterministic {
            sequence: RefCell::new(bytes.into_iter().cycle()),
        }
    }
}

impl WasiRandom for Deterministic {
    fn get(&self, buf: &mut [u8]) -> Result<(), Error> {
        let mut s = self.sequence.borrow_mut();
        for b in buf.iter_mut() {
            *b = s.next().expect("infinite sequence");
        }
        Ok(())
    }
}
