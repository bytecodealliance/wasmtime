use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{self, Seek, SeekFrom, Write},
};

fn main() -> Result<(), Box<dyn Error>> {
    {
        let mut file = File::open("bar.txt")?;

        assert_eq!(27, file.metadata()?.len());

        assert_eq!(
            "And stood awhile in thought",
            &io::read_to_string(&mut file)?
        );

        file.seek(SeekFrom::Start(11))?;

        assert_eq!("while in thought", &io::read_to_string(&mut file)?);

        assert!(
            file.write_all(b"Did gyre and gimble in the wabe;\n")
                .is_err()
        );
    }

    assert!(OpenOptions::new().append(true).open("bar.txt").is_err());
    assert!(File::create("new.txt").is_err());
    assert!(fs::create_dir("sub2").is_err());
    assert!(fs::rename("bar.txt", "baz.txt").is_err());
    assert!(fs::remove_file("bar.txt").is_err());
    assert!(fs::remove_dir("sub").is_err());

    Ok(())
}
