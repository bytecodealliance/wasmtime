/// Options and flags which can be used to configure how a file is opened.
///
/// This corresponds to [`std::fs::OpenOptions`].
///
/// Note that this `OpenOptions` has no `open` method. To open a file with
/// an `OptionOptions`, you must first obtain a [`Dir`] containing the file, and
/// then call [`Dir::open_file_with`].
///
/// [`std::fs::OpenOptions`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html
/// [`Dir`]: struct.Dir.html
/// [`Dir::open_file_with`]: struct.Dir.html#method.open_file_with
pub struct OpenOptions {
    pub(crate) read: bool,
    pub(crate) write: bool,
    pub(crate) append: bool,
    pub(crate) truncate: bool,
    pub(crate) create: bool,
    pub(crate) create_new: bool,
}

impl OpenOptions {
    /// Creates a blank new set of options ready for configuration.
    ///
    /// This corresponds to [`std::fs::OpenOptions::new`].
    ///
    /// [`std::fs::OpenOptions::new`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.new
    pub fn new() -> Self {
        Self {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
        }
    }

    /// Sets the option for read access.
    ///
    /// This corresponds to [`std::fs::OpenOptions::read`].
    ///
    /// [`std::fs::OpenOptions::read`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.read
    pub fn read(&mut self, read: bool) -> &mut Self {
        self.read = read;
        self
    }

    /// Sets the option for write access.
    ///
    /// This corresponds to [`std::fs::OpenOptions::write`].
    ///
    /// [`std::fs::OpenOptions::write`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.write
    pub fn write(&mut self, write: bool) -> &mut Self {
        self.write = write;
        self
    }

    /// Sets the option for the append mode.
    ///
    /// This corresponds to [`std::fs::OpenOptions::append`].
    ///
    /// [`std::fs::OpenOptions::append`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.append
    pub fn append(&mut self, append: bool) -> &mut Self {
        self.append = append;
        self
    }

    /// Sets the option for truncating a previous file.
    ///
    /// This corresponds to [`std::fs::OpenOptions::truncate`].
    ///
    /// [`std::fs::OpenOptions::truncate`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.truncate
    pub fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.truncate = truncate;
        self
    }

    /// Sets the option to create a new file.
    ///
    /// This corresponds to [`std::fs::OpenOptions::create`].
    ///
    /// [`std::fs::OpenOptions::create`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.create
    pub fn create(&mut self, create: bool) -> &mut Self {
        self.create = create;
        self
    }

    /// Sets the option to always create a new file.
    ///
    /// This corresponds to [`std::fs::OpenOptions::create_new`].
    ///
    /// [`std::fs::OpenOptions::create_new`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.create_new
    pub fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.create_new = create_new;
        self
    }
}

// TODO: Functions from OpenOptionsExt?
