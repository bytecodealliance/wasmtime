use std::{
    error::Error,
    fs::File,
    io::{self, Seek, SeekFrom},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut file = File::open("bar.txt")?;

    assert_eq!(27, file.metadata()?.len());

    assert_eq!(
        "And stood awhile in thought",
        &io::read_to_string(&mut file)?
    );

    file.seek(SeekFrom::Start(11))?;

    assert_eq!("while in thought", &io::read_to_string(&mut file)?);

    Ok(())
}
