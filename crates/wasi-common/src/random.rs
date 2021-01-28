use cap_rand::RngCore;

/// Implement `WasiRandom` using a deterministic cycle of bytes.
pub struct Deterministic {
    cycle: std::iter::Cycle<std::vec::IntoIter<u8>>,
}

impl Deterministic {
    pub fn new(bytes: Vec<u8>) -> Self {
        Deterministic {
            cycle: bytes.into_iter().cycle(),
        }
    }
}

impl RngCore for Deterministic {
    fn next_u32(&mut self) -> u32 {
        todo!()
    }
    fn next_u64(&mut self) -> u64 {
        todo!()
    }
    fn fill_bytes(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.cycle.next().expect("infinite sequence");
        }
    }
    fn try_fill_bytes(&mut self, buf: &mut [u8]) -> Result<(), cap_rand::Error> {
        self.fill_bytes(buf);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn deterministic() {
        let mut det = Deterministic::new(vec![1, 2, 3, 4]);
        let mut buf = vec![0; 1024];
        det.try_fill_bytes(&mut buf).expect("get randomness");
        for (ix, b) in buf.iter().enumerate() {
            assert_eq!(*b, (ix % 4) as u8 + 1)
        }
    }
}
