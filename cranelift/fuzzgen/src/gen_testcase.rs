use arbitrary::Unstructured;

use cranelift_fuzzgen::FuzzGen;
use rand::{thread_rng, Rng};

fn main() {
    let data = if let Some(file) = std::env::args().nth(1) {
        println!("; input file: {}", file);
        std::fs::read(file).unwrap()
    } else {
        println!("; no input file, generating random bytes");
        let mut data = [0u8; 4096];
        thread_rng().fill(&mut data[..]);
        Vec::from(data)
    };
    let mut u = Unstructured::new(&data[..]);

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
