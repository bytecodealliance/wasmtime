fn main() {
    // Ensure that new files in the filetests directory cause a rebuild.
    println!("cargo:rerun-if-changed=filetests");
}
