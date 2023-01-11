fn main() -> std::io::Result<()> {
    let file = std::fs::File::open("bar.txt")?;
    file.sync_all()?;
    file.sync_data()?;

    let dir = std::fs::File::open(".")?;
    dir.sync_all()?;
    dir.sync_data()?;

    Ok(())
}
