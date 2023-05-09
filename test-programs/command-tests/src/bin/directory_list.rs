use anyhow::{Context, Error};
use std::{collections::HashSet, fs, path::PathBuf};

fn main() -> Result<(), Error> {
    assert_eq!(
        ["/foo.txt", "/bar.txt", "/baz.txt", "/sub"]
            .into_iter()
            .map(PathBuf::from)
            .collect::<HashSet<_>>(),
        fs::read_dir("/")
            .context("read_dir /")?
            .map(|r| r.map(|d| d.path()))
            .collect::<Result<_, _>>()
            .context("elem in /")?
    );

    assert_eq!(
        ["/sub/wow.txt", "/sub/yay.txt"]
            .into_iter()
            .map(PathBuf::from)
            .collect::<HashSet<_>>(),
        fs::read_dir("/sub")
            .context("read_dir /sub")?
            .map(|r| r.map(|d| d.path()))
            .collect::<Result<_, _>>()
            .context("elem in /sub")?
    );

    Ok(())
}
