//! Utility functions.

use std::path::Path;
use std::fs::File;
use std::io::{Result, Read};

/// Read an entire file into a string.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = try!(File::open(path));
    let mut buffer = String::new();
    try!(file.read_to_string(&mut buffer));
    Ok(buffer)
}
