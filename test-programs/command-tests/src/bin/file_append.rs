use std::{
    error::Error,
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut file = OpenOptions::new().append(true).open("bar.txt")?;

    file.write_all(b"Did gyre and gimble in the wabe;\n")
        .unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(b"All mimsy were the borogoves,\n").unwrap();
    file.write_all(b"And the mome raths outgrabe.\n").unwrap();

    Ok(())
}
