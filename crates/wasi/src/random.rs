use cap_rand::RngCore;

/// Implement `insecure-random` using a deterministic cycle of bytes.
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
        let b0 = self.cycle.next().expect("infinite sequence");
        let b1 = self.cycle.next().expect("infinite sequence");
        let b2 = self.cycle.next().expect("infinite sequence");
        let b3 = self.cycle.next().expect("infinite sequence");
        ((b0 as u32) << 24) + ((b1 as u32) << 16) + ((b2 as u32) << 8) + (b3 as u32)
    }
    fn next_u64(&mut self) -> u64 {
        let w0 = self.next_u32();
        let w1 = self.next_u32();
        ((w0 as u64) << 32) + (w1 as u64)
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

pub fn thread_rng() -> Box<dyn RngCore + Send> {
    use cap_rand::{Rng, SeedableRng};
    let mut rng = cap_rand::thread_rng(cap_rand::ambient_authority());
    Box::new(cap_rand::rngs::StdRng::from_seed(rng.r#gen()))
}
