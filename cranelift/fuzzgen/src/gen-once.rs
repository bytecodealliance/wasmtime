use arbitrary::Unstructured;

use cranelift_fuzzgen::FuzzGen;
use rand::{thread_rng, Rng};

fn main() {
    // let mut u = Unstructured::new(&[0, 0, 2u8]);
    let mut data = [0u8; 128];
    thread_rng().fill(&mut data[..]);
    let mut u = Unstructured::new(&data);

    let mut fuzzgen = FuzzGen::new(&mut u);
    let testcase = fuzzgen.generate_test().unwrap();

    println!("{}", testcase.func.display(None));
    for input in testcase.inputs {
        let fmt_inputs = input
            .iter()
            .map(|i| format!("{}", i))
            .collect::<Vec<String>>()
            .join(", ");

        println!("; run: %{}({}) == ?", testcase.func.name, fmt_inputs);
    }
}
