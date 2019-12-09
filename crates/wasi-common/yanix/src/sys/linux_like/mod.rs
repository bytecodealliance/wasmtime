use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_os = "emscripten")] {
        pub(crate) mod emscripten;
    } else {
        pub(crate) mod linux;
    }
}

pub(crate) mod dir;
pub(crate) mod fadvise;
pub(crate) mod file;
