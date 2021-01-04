use crate::Error;
use std::cell::RefCell;

pub trait WasiRandom {
    fn get(&self, buf: &mut [u8]) -> Result<(), Error>;
}

/// Implement `WasiRandom` using the `getrandom` crate, which selects your system's best entropy
/// source.
pub struct GetRandom;

impl WasiRandom for GetRandom {
    fn get(&self, buf: &mut [u8]) -> Result<(), Error> {
        getrandom::getrandom(buf)?;
        Ok(())
    }
}

/// Implement `WasiRandom` using a deterministic cycle of bytes.
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn deterministic() {
        let det = Deterministic::new(vec![1, 2, 3, 4]);
        let mut buf = vec![0; 1024];
        det.get(&mut buf).expect("get randomness");
        for (ix, b) in buf.iter().enumerate() {
            assert_eq!(*b, (ix % 4) as u8 + 1)
        }
    }
}
