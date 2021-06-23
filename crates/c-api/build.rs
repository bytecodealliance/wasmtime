fn main() {
    // Set SONAME in shared library on Linux.
    //

    // A missing SONAME hinders full C-ABI compatibility.
    // Libraries without SONAME can produce unwanted NEEDED entries in
    // executables linked to this shared library.
    // Libraries without SONAME need special treatment in the CMake
    // build system (i.e. IMPORTED_NO_SONAME).
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libwasmtime.so");
    }
}
