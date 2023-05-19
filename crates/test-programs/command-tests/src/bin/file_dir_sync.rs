fn main() -> std::io::Result<()> {
    let file = std::fs::File::open("bar.txt")?;
    file.sync_all()?;
    file.sync_data()?;

    /*
     * TODO: Support opening directories with `File::open` on Windows.
    let dir = cap_std::fs::Dir::open(".")?;
    dir.sync_all()?;
    dir.sync_data()?;
    */

    Ok(())
}
