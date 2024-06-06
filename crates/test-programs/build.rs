fn main() {
    println!("cargo:rustc-link-arg-bin=dwarf_imported_memory=--import-memory");
    println!("cargo:rustc-link-arg-bin=dwarf_imported_memory=--export-memory");
    println!("cargo:rustc-link-arg-bin=dwarf_shared_memory=--no-check-features");
    println!("cargo:rustc-link-arg-bin=dwarf_shared_memory=--shared-memory");
}
