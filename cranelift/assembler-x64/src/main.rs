//! Generate `assembler-definitions.isle` for debugging purposes.

const PATH: &str = "assembler-definitions.isle";

fn main() {
    std::fs::write(
        PATH,
        cranelift_assembler_x64::assembler_definitions_isle_contents(),
    )
    .unwrap();

    println!("Successfully wrote generated isle to {PATH}.");
}
