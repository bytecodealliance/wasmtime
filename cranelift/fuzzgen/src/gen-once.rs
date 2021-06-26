use arbitrary::Unstructured;

use cranelift_fuzzgen::FuzzGen;
use rand::{thread_rng, Rng};

fn main() {
    let mut u = Unstructured::new(&[171u8, 171, 188, 56, 56, 56, 171, 56]);
    // let mut data = [0u8; 1024];
    // thread_rng().fill(&mut data[..]);
    // let mut u = Unstructured::new(&data);

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
