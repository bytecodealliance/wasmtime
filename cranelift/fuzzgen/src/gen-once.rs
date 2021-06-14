use arbitrary::Unstructured;

use cranelift_fuzzgen::FuzzGen;
use rand::{thread_rng, Rng};

fn main() {
    // let mut u = Unstructured::new(&[2, 5, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4]);
    let mut data = [0u8; 128];
    thread_rng().fill(&mut data[..]);

    let mut u = Unstructured::new(&data);

    let mut fuzzgen = FuzzGen::new(&mut u);
    let func = fuzzgen.generate_function().unwrap();

    println!("{}", func.display(None));
}
