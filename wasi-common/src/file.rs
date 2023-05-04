pub struct File {
    file: cap_std::fs::File,
    read_only: bool,
}

impl File {
    pub fn new(file: cap_std::fs::File) -> Self {
        Self {
            file,
            read_only: false,
        }
    }
    pub fn read_only(file: cap_std::fs::File) -> Self {
        Self {
            file,
            read_only: true,
        }
    }
}
