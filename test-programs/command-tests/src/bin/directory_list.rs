use std::{collections::HashSet, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        ["/foo.txt", "/bar.txt", "/baz.txt", "/sub"]
            .into_iter()
            .map(PathBuf::from)
            .collect::<HashSet<_>>(),
        fs::read_dir("/")?
            .map(|r| r.map(|d| d.path()))
            .collect::<Result<_, _>>()?
    );

    assert_eq!(
        ["/sub/wow.txt", "/sub/yay.txt"]
            .into_iter()
            .map(PathBuf::from)
            .collect::<HashSet<_>>(),
        fs::read_dir("/sub")?
            .map(|r| r.map(|d| d.path()))
            .collect::<Result<_, _>>()?
    );

    Ok(())
}
