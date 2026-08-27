use std::ffi::OsStr;
use std::path::{Component, PathBuf};

/// Utility for collecting the canonical path components.
pub(super) struct CanonicalPath<'path_buf> {
    /// If the user requested a canonical path, a reference to the `PathBuf` to
    /// write it to.
    path: Option<&'path_buf mut PathBuf>,
}

impl<'path_buf> CanonicalPath<'path_buf> {
    pub(super) fn new(path: Option<&'path_buf mut PathBuf>) -> Self {
        Self { path }
    }

    pub(super) fn push(&mut self, one: &OsStr) {
        if let Some(path) = &mut self.path {
            path.push(one)
        }
    }

    pub(super) fn pop(&mut self) -> bool {
        if let Some(path) = &mut self.path {
            path.pop()
        } else {
            true
        }
    }

    /// The complete canonical path has been scanned. Set `path` to `None`
    /// so that it isn't cleared when `self` is dropped.
    pub(super) fn complete(&mut self) {
        // Replace "" with ".", since "" as a relative path is interpreted as
        // an error.
        if let Some(path) = &mut self.path {
            if path.as_os_str().is_empty() {
                path.push(Component::CurDir);
            }
            self.path = None;
        }
    }
}

impl<'path_buf> Drop for CanonicalPath<'path_buf> {
    fn drop(&mut self) {
        // If `self.path` is still `Some` here, it means that we haven't called
        // `complete()` yet, meaning the `CanonicalPath` is being dropped
        // before the complete path has been processed. In that case, clear
        // `path` to indicate that we weren't able to obtain a complete path.
        if let Some(path) = &mut self.path {
            path.clear();
            self.path = None;
        }
    }
}
