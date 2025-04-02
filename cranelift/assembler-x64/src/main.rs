//! Print the path to the generated code.

fn main() {
    for path in cranelift_assembler_x64::generated_files() {
        println!("{}", path.display());
    }
}
