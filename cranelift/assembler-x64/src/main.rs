//! Print the contents of the generated `assembler_definitions.isle`.

fn main() {
    println!(
        "{}",
        std::str::from_utf8(&cranelift_assembler_x64::assembler_definitions_isle_contents())
            .unwrap()
    );
}
