fn main() {
    println!("cargo:rustc-link-arg-bin=dwarf_imported_memory=--import-memory");
    println!("cargo:rustc-link-arg-bin=dwarf_imported_memory=--export-memory");
}
