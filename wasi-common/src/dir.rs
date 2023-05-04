pub struct Dir {
    dir: cap_std::fs::Dir,
    read_only: bool,
}

impl Dir {
    pub fn new(dir: cap_std::fs::Dir) -> Self {
        Dir {
            dir,
            read_only: false,
        }
    }
    pub fn read_only(dir: cap_std::fs::Dir) -> Self {
        Dir {
            dir,
            read_only: true,
        }
    }
}
