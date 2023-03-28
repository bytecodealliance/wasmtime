#[allow(clippy::all)]
pub mod random {
    #[allow(clippy::all)]
    /// Return `len` cryptographically-secure pseudo-random bytes.
    ///
    /// This function must produce data from an adequately seeded
    /// cryptographically-secure pseudo-random number generator (CSPRNG), so it
    /// must not block, from the perspective of the calling program, and the
    /// returned data is always unpredictable.
    ///
    /// This function must always return fresh pseudo-random data. Deterministic
    /// environments must omit this function, rather than implementing it with
    /// deterministic data.
    pub fn get_random_bytes(len: u64) -> wit_bindgen::rt::vec::Vec<u8> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "random")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "get-random-bytes")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "random_get-random-bytes")]
                fn wit_import(_: i64, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i64(len), ptr0);
            let len1 = *((ptr0 + 4) as *const i32) as usize;
            Vec::from_raw_parts(*((ptr0 + 0) as *const i32) as *mut _, len1, len1)
        }
    }
    #[allow(clippy::all)]
    /// Return a cryptographically-secure pseudo-random `u64` value.
    ///
    /// This function returns the same type of pseudo-random data as
    /// `get-random-bytes`, represented as a `u64`.
    pub fn get_random_u64() -> u64 {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "random")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "get-random-u64")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "random_get-random-u64")]
                fn wit_import() -> i64;
            }
            let ret = wit_import();
            ret as u64
        }
    }
    #[allow(clippy::all)]
    /// Return a 128-bit value that may contain a pseudo-random value.
    ///
    /// The returned value is not required to be computed from a CSPRNG, and may
    /// even be entirely deterministic. Host implementations are encouraged to
    /// provide pseudo-random values to any program exposed to
    /// attacker-controlled content, to enable DoS protection built into many
    /// languages' hash-map implementations.
    ///
    /// This function is intended to only be called once, by a source language
    /// to initialize Denial Of Service (DoS) protection in its hash-map
    /// implementation.
    ///
    /// # Expected future evolution
    ///
    /// This will likely be changed to a value import, to prevent it from being
    /// called multiple times and potentially used for purposes other than DoS
    /// protection.
    pub fn insecure_random() -> (u64, u64) {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(8))]
            struct RetArea([u8; 16]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "random")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "insecure-random")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "random_insecure-random")]
                fn wit_import(_: i32);
            }
            wit_import(ptr0);
            (
                *((ptr0 + 0) as *const i64) as u64,
                *((ptr0 + 8) as *const i64) as u64,
            )
        }
    }
}

#[allow(clippy::all)]
pub mod console {
    /// A log level, describing a kind of message.
    #[repr(u8)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Level {
        /// Describes messages about the values of variables and the flow of
        /// control within a program.
        Trace,
        /// Describes messages likely to be of interest to someone debugging a
        /// program.
        Debug,
        /// Describes messages likely to be of interest to someone monitoring a
        /// program.
        Info,
        /// Describes messages indicating hazardous situations.
        Warn,
        /// Describes messages indicating serious errors.
        Error,
    }
    impl core::fmt::Debug for Level {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Level::Trace => f.debug_tuple("Level::Trace").finish(),
                Level::Debug => f.debug_tuple("Level::Debug").finish(),
                Level::Info => f.debug_tuple("Level::Info").finish(),
                Level::Warn => f.debug_tuple("Level::Warn").finish(),
                Level::Error => f.debug_tuple("Level::Error").finish(),
            }
        }
    }
    #[allow(clippy::all)]
    /// Emit a log message.
    ///
    /// A log message has a `level` describing what kind of message is being
    /// sent, a context, which is an uninterpreted string meant to help
    /// consumers group similar messages, and a string containing the message
    /// text.
    pub fn log(level: Level, context: &str, message: &str) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            let vec0 = context;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;
            let vec1 = message;
            let ptr1 = vec1.as_ptr() as i32;
            let len1 = vec1.len() as i32;

            #[link(wasm_import_module = "console")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "log")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "console_log")]
                fn wit_import(_: i32, _: i32, _: i32, _: i32, _: i32);
            }
            wit_import(
                match level {
                    Level::Trace => 0,
                    Level::Debug => 1,
                    Level::Info => 2,
                    Level::Warn => 3,
                    Level::Error => 4,
                },
                ptr0,
                len0,
                ptr1,
                len1,
            );
        }
    }
}

#[allow(clippy::all)]
pub mod poll {
    /// A "pollable" handle.
    ///
    /// This is conceptually represents a `stream<_, _>`, or in other words,
    /// a stream that one can wait on, repeatedly, but which does not itself
    /// produce any data. It's temporary scaffolding until component-model's
    /// async features are ready.
    ///
    /// And at present, it is a `u32` instead of being an actual handle, until
    /// the wit-bindgen implementation of handles and resources is ready.
    ///
    /// `pollable` lifetimes are not automatically managed. Users must ensure
    /// that they do not outlive the resource they reference.
    ///
    /// This [represents a resource](https://github.com/WebAssembly/WASI/blob/main/docs/WitInWasi.md#Resources).
    pub type Pollable = u32;
    #[allow(clippy::all)]
    /// Dispose of the specified `pollable`, after which it may no longer
    /// be used.
    pub fn drop_pollable(this: Pollable) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "poll")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "drop-pollable")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "poll_drop-pollable")]
                fn wit_import(_: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(this));
        }
    }
    #[allow(clippy::all)]
    /// Poll for completion on a set of pollables.
    ///
    /// The "oneoff" in the name refers to the fact that this function must do a
    /// linear scan through the entire list of subscriptions, which may be
    /// inefficient if the number is large and the same subscriptions are used
    /// many times. In the future, this is expected to be obsoleted by the
    /// component model async proposal, which will include a scalable waiting
    /// facility.
    ///
    /// Note that the return type would ideally be `list<bool>`, but that would
    /// be more difficult to polyfill given the current state of `wit-bindgen`.
    /// See <https://github.com/bytecodealliance/preview2-prototyping/pull/11#issuecomment-1329873061>
    /// for details.  For now, we use zero to mean "not ready" and non-zero to
    /// mean "ready".
    pub fn poll_oneoff(in_: &[Pollable]) -> wit_bindgen::rt::vec::Vec<u8> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let vec0 = in_;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;
            let ptr1 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "poll")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "poll-oneoff")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "poll_poll-oneoff")]
                fn wit_import(_: i32, _: i32, _: i32);
            }
            wit_import(ptr0, len0, ptr1);
            let len2 = *((ptr1 + 4) as *const i32) as usize;
            Vec::from_raw_parts(*((ptr1 + 0) as *const i32) as *mut _, len2, len2)
        }
    }
}

#[allow(clippy::all)]
pub mod streams {
    pub type Pollable = super::poll::Pollable;
    /// An error type returned from a stream operation. Currently this
    /// doesn't provide any additional information.
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct StreamError {}
    impl core::fmt::Debug for StreamError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("StreamError").finish()
        }
    }
    impl core::fmt::Display for StreamError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{:?}", self)
        }
    }
    impl std::error::Error for StreamError {}
    /// An output bytestream. In the future, this will be replaced by handle
    /// types.
    ///
    /// This conceptually represents a `stream<u8, _>`. It's temporary
    /// scaffolding until component-model's async features are ready.
    ///
    /// `output-stream`s are *non-blocking* to the extent practical on
    /// underlying platforms. Except where specified otherwise, I/O operations also
    /// always return promptly, after the number of bytes that can be written
    /// promptly, which could even be zero. To wait for the stream to be ready to
    /// accept data, the `subscribe-to-output-stream` function to obtain a
    /// `pollable` which can be polled for using `wasi_poll`.
    ///
    /// And at present, it is a `u32` instead of being an actual handle, until
    /// the wit-bindgen implementation of handles and resources is ready.
    ///
    /// This [represents a resource](https://github.com/WebAssembly/WASI/blob/main/docs/WitInWasi.md#Resources).
    pub type OutputStream = u32;
    /// An input bytestream. In the future, this will be replaced by handle
    /// types.
    ///
    /// This conceptually represents a `stream<u8, _>`. It's temporary
    /// scaffolding until component-model's async features are ready.
    ///
    /// `input-stream`s are *non-blocking* to the extent practical on underlying
    /// platforms. I/O operations always return promptly; if fewer bytes are
    /// promptly available than requested, they return the number of bytes promptly
    /// available, which could even be zero. To wait for data to be available,
    /// use the `subscribe-to-input-stream` function to obtain a `pollable` which
    /// can be polled for using `wasi_poll`.
    ///
    /// And at present, it is a `u32` instead of being an actual handle, until
    /// the wit-bindgen implementation of handles and resources is ready.
    ///
    /// This [represents a resource](https://github.com/WebAssembly/WASI/blob/main/docs/WitInWasi.md#Resources).
    pub type InputStream = u32;
    #[allow(clippy::all)]
    /// Read bytes from a stream.
    ///
    /// This function returns a list of bytes containing the data that was
    /// read, along with a bool indicating whether the end of the stream
    /// was reached. The returned list will contain up to `len` bytes; it
    /// may return fewer than requested, but not more.
    ///
    /// Once a stream has reached the end, subsequent calls to read or
    /// `skip` will always report end-of-stream rather than producing more
    /// data.
    ///
    /// If `len` is 0, it represents a request to read 0 bytes, which should
    /// always succeed, assuming the stream hasn't reached its end yet, and
    /// return an empty list.
    ///
    /// The len here is a `u64`, but some callees may not be able to allocate
    /// a buffer as large as that would imply.
    /// FIXME: describe what happens if allocation fails.
    pub fn read(
        this: InputStream,
        len: u64,
    ) -> Result<(wit_bindgen::rt::vec::Vec<u8>, bool), StreamError> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 16]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "streams")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "read")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "streams_read")]
                fn wit_import(_: i32, _: i64, _: i32);
            }
            wit_import(
                wit_bindgen::rt::as_i32(this),
                wit_bindgen::rt::as_i64(len),
                ptr0,
            );
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => Ok({
                    let len1 = *((ptr0 + 8) as *const i32) as usize;

                    (
                        Vec::from_raw_parts(*((ptr0 + 4) as *const i32) as *mut _, len1, len1),
                        match i32::from(*((ptr0 + 12) as *const u8)) {
                            0 => false,
                            1 => true,
                            _ => panic!("invalid bool discriminant"),
                        },
                    )
                }),
                1 => Err(StreamError {}),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    /// Skip bytes from a stream.
    ///
    /// This is similar to the `read` function, but avoids copying the
    /// bytes into the instance.
    ///
    /// Once a stream has reached the end, subsequent calls to read or
    /// `skip` will always report end-of-stream rather than producing more
    /// data.
    ///
    /// This function returns the number of bytes skipped, along with a bool
    /// indicating whether the end of the stream was reached. The returned
    /// value will be at most `len`; it may be less.
    pub fn skip(this: InputStream, len: u64) -> Result<(u64, bool), StreamError> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(8))]
            struct RetArea([u8; 24]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "streams")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "skip")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "streams_skip")]
                fn wit_import(_: i32, _: i64, _: i32);
            }
            wit_import(
                wit_bindgen::rt::as_i32(this),
                wit_bindgen::rt::as_i64(len),
                ptr0,
            );
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => Ok((
                    *((ptr0 + 8) as *const i64) as u64,
                    match i32::from(*((ptr0 + 16) as *const u8)) {
                        0 => false,
                        1 => true,
                        _ => panic!("invalid bool discriminant"),
                    },
                )),
                1 => Err(StreamError {}),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    /// Create a `pollable` which will resolve once either the specified stream
    /// has bytes available to read or the other end of the stream has been
    /// closed.
    pub fn subscribe_to_input_stream(this: InputStream) -> Pollable {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "streams")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "subscribe-to-input-stream")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "streams_subscribe-to-input-stream"
                )]
                fn wit_import(_: i32) -> i32;
            }
            let ret = wit_import(wit_bindgen::rt::as_i32(this));
            ret as u32
        }
    }
    #[allow(clippy::all)]
    /// Dispose of the specified `input-stream`, after which it may no longer
    /// be used.
    pub fn drop_input_stream(this: InputStream) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "streams")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "drop-input-stream")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "streams_drop-input-stream")]
                fn wit_import(_: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(this));
        }
    }
    #[allow(clippy::all)]
    /// Write bytes to a stream.
    ///
    /// This function returns a `u64` indicating the number of bytes from
    /// `buf` that were written; it may be less than the full list.
    pub fn write(this: OutputStream, buf: &[u8]) -> Result<u64, StreamError> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(8))]
            struct RetArea([u8; 16]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let vec0 = buf;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;
            let ptr1 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "streams")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "write")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "streams_write")]
                fn wit_import(_: i32, _: i32, _: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(this), ptr0, len0, ptr1);
            match i32::from(*((ptr1 + 0) as *const u8)) {
                0 => Ok(*((ptr1 + 8) as *const i64) as u64),
                1 => Err(StreamError {}),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    /// Write multiple zero bytes to a stream.
    ///
    /// This function returns a `u64` indicating the number of zero bytes
    /// that were written; it may be less than `len`.
    pub fn write_zeroes(this: OutputStream, len: u64) -> Result<u64, StreamError> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(8))]
            struct RetArea([u8; 16]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "streams")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "write-zeroes")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "streams_write-zeroes")]
                fn wit_import(_: i32, _: i64, _: i32);
            }
            wit_import(
                wit_bindgen::rt::as_i32(this),
                wit_bindgen::rt::as_i64(len),
                ptr0,
            );
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => Ok(*((ptr0 + 8) as *const i64) as u64),
                1 => Err(StreamError {}),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    /// Read from one stream and write to another.
    ///
    /// This function returns the number of bytes transferred; it may be less
    /// than `len`.
    ///
    /// Unlike other I/O functions, this function blocks until all the data
    /// read from the input stream has been written to the output stream.
    pub fn splice(
        this: OutputStream,
        src: InputStream,
        len: u64,
    ) -> Result<(u64, bool), StreamError> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(8))]
            struct RetArea([u8; 24]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "streams")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "splice")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "streams_splice")]
                fn wit_import(_: i32, _: i32, _: i64, _: i32);
            }
            wit_import(
                wit_bindgen::rt::as_i32(this),
                wit_bindgen::rt::as_i32(src),
                wit_bindgen::rt::as_i64(len),
                ptr0,
            );
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => Ok((
                    *((ptr0 + 8) as *const i64) as u64,
                    match i32::from(*((ptr0 + 16) as *const u8)) {
                        0 => false,
                        1 => true,
                        _ => panic!("invalid bool discriminant"),
                    },
                )),
                1 => Err(StreamError {}),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    /// Forward the entire contents of an input stream to an output stream.
    ///
    /// This function repeatedly reads from the input stream and writes
    /// the data to the output stream, until the end of the input stream
    /// is reached, or an error is encountered.
    ///
    /// Unlike other I/O functions, this function blocks until the end
    /// of the input stream is seen and all the data has been written to
    /// the output stream.
    ///
    /// This function returns the number of bytes transferred.
    pub fn forward(this: OutputStream, src: InputStream) -> Result<u64, StreamError> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(8))]
            struct RetArea([u8; 16]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "streams")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "forward")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "streams_forward")]
                fn wit_import(_: i32, _: i32, _: i32);
            }
            wit_import(
                wit_bindgen::rt::as_i32(this),
                wit_bindgen::rt::as_i32(src),
                ptr0,
            );
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => Ok(*((ptr0 + 8) as *const i64) as u64),
                1 => Err(StreamError {}),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    /// Create a `pollable` which will resolve once either the specified stream
    /// is ready to accept bytes or the other end of the stream has been closed.
    pub fn subscribe_to_output_stream(this: OutputStream) -> Pollable {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "streams")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "subscribe-to-output-stream")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "streams_subscribe-to-output-stream"
                )]
                fn wit_import(_: i32) -> i32;
            }
            let ret = wit_import(wit_bindgen::rt::as_i32(this));
            ret as u32
        }
    }
    #[allow(clippy::all)]
    /// Dispose of the specified `output-stream`, after which it may no longer
    /// be used.
    pub fn drop_output_stream(this: OutputStream) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "streams")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "drop-output-stream")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "streams_drop-output-stream")]
                fn wit_import(_: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(this));
        }
    }
}

#[allow(clippy::all)]
pub mod types {
    pub type InputStream = super::streams::InputStream;
    pub type OutputStream = super::streams::OutputStream;
    pub type Pollable = super::poll::Pollable;
    pub type StatusCode = u16;
    #[derive(Clone)]
    pub enum SchemeParam<'a> {
        Http,
        Https,
        Other(&'a str),
    }
    impl<'a> core::fmt::Debug for SchemeParam<'a> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                SchemeParam::Http => f.debug_tuple("SchemeParam::Http").finish(),
                SchemeParam::Https => f.debug_tuple("SchemeParam::Https").finish(),
                SchemeParam::Other(e) => f.debug_tuple("SchemeParam::Other").field(e).finish(),
            }
        }
    }
    #[derive(Clone)]
    pub enum SchemeResult {
        Http,
        Https,
        Other(wit_bindgen::rt::string::String),
    }
    impl core::fmt::Debug for SchemeResult {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                SchemeResult::Http => f.debug_tuple("SchemeResult::Http").finish(),
                SchemeResult::Https => f.debug_tuple("SchemeResult::Https").finish(),
                SchemeResult::Other(e) => f.debug_tuple("SchemeResult::Other").field(e).finish(),
            }
        }
    }
    pub type ResponseOutparam = u32;
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct RequestOptions {
        pub connect_timeout_ms: Option<u32>,
        pub first_byte_timeout_ms: Option<u32>,
        pub between_bytes_timeout_ms: Option<u32>,
    }
    impl core::fmt::Debug for RequestOptions {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("RequestOptions")
                .field("connect-timeout-ms", &self.connect_timeout_ms)
                .field("first-byte-timeout-ms", &self.first_byte_timeout_ms)
                .field("between-bytes-timeout-ms", &self.between_bytes_timeout_ms)
                .finish()
        }
    }
    pub type OutgoingStream = OutputStream;
    pub type OutgoingResponse = u32;
    pub type OutgoingRequest = u32;
    #[derive(Clone)]
    pub enum MethodParam<'a> {
        Get,
        Head,
        Post,
        Put,
        Delete,
        Connect,
        Options,
        Trace,
        Patch,
        Other(&'a str),
    }
    impl<'a> core::fmt::Debug for MethodParam<'a> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                MethodParam::Get => f.debug_tuple("MethodParam::Get").finish(),
                MethodParam::Head => f.debug_tuple("MethodParam::Head").finish(),
                MethodParam::Post => f.debug_tuple("MethodParam::Post").finish(),
                MethodParam::Put => f.debug_tuple("MethodParam::Put").finish(),
                MethodParam::Delete => f.debug_tuple("MethodParam::Delete").finish(),
                MethodParam::Connect => f.debug_tuple("MethodParam::Connect").finish(),
                MethodParam::Options => f.debug_tuple("MethodParam::Options").finish(),
                MethodParam::Trace => f.debug_tuple("MethodParam::Trace").finish(),
                MethodParam::Patch => f.debug_tuple("MethodParam::Patch").finish(),
                MethodParam::Other(e) => f.debug_tuple("MethodParam::Other").field(e).finish(),
            }
        }
    }
    #[derive(Clone)]
    pub enum MethodResult {
        Get,
        Head,
        Post,
        Put,
        Delete,
        Connect,
        Options,
        Trace,
        Patch,
        Other(wit_bindgen::rt::string::String),
    }
    impl core::fmt::Debug for MethodResult {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                MethodResult::Get => f.debug_tuple("MethodResult::Get").finish(),
                MethodResult::Head => f.debug_tuple("MethodResult::Head").finish(),
                MethodResult::Post => f.debug_tuple("MethodResult::Post").finish(),
                MethodResult::Put => f.debug_tuple("MethodResult::Put").finish(),
                MethodResult::Delete => f.debug_tuple("MethodResult::Delete").finish(),
                MethodResult::Connect => f.debug_tuple("MethodResult::Connect").finish(),
                MethodResult::Options => f.debug_tuple("MethodResult::Options").finish(),
                MethodResult::Trace => f.debug_tuple("MethodResult::Trace").finish(),
                MethodResult::Patch => f.debug_tuple("MethodResult::Patch").finish(),
                MethodResult::Other(e) => f.debug_tuple("MethodResult::Other").field(e).finish(),
            }
        }
    }
    pub type IncomingStream = InputStream;
    pub type IncomingResponse = u32;
    pub type IncomingRequest = u32;
    pub type FutureIncomingResponse = u32;
    pub type Fields = u32;
    pub type Trailers = Fields;
    pub type Headers = Fields;
    #[derive(Clone)]
    pub enum ErrorParam<'a> {
        InvalidUrl(&'a str),
        TimeoutError(&'a str),
        ProtocolError(&'a str),
        UnexpectedError(&'a str),
    }
    impl<'a> core::fmt::Debug for ErrorParam<'a> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                ErrorParam::InvalidUrl(e) => {
                    f.debug_tuple("ErrorParam::InvalidUrl").field(e).finish()
                }
                ErrorParam::TimeoutError(e) => {
                    f.debug_tuple("ErrorParam::TimeoutError").field(e).finish()
                }
                ErrorParam::ProtocolError(e) => {
                    f.debug_tuple("ErrorParam::ProtocolError").field(e).finish()
                }
                ErrorParam::UnexpectedError(e) => f
                    .debug_tuple("ErrorParam::UnexpectedError")
                    .field(e)
                    .finish(),
            }
        }
    }
    impl<'a> core::fmt::Display for ErrorParam<'a> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl<'a> std::error::Error for ErrorParam<'a> {}
    #[derive(Clone)]
    pub enum ErrorResult {
        InvalidUrl(wit_bindgen::rt::string::String),
        TimeoutError(wit_bindgen::rt::string::String),
        ProtocolError(wit_bindgen::rt::string::String),
        UnexpectedError(wit_bindgen::rt::string::String),
    }
    impl core::fmt::Debug for ErrorResult {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                ErrorResult::InvalidUrl(e) => {
                    f.debug_tuple("ErrorResult::InvalidUrl").field(e).finish()
                }
                ErrorResult::TimeoutError(e) => {
                    f.debug_tuple("ErrorResult::TimeoutError").field(e).finish()
                }
                ErrorResult::ProtocolError(e) => f
                    .debug_tuple("ErrorResult::ProtocolError")
                    .field(e)
                    .finish(),
                ErrorResult::UnexpectedError(e) => f
                    .debug_tuple("ErrorResult::UnexpectedError")
                    .field(e)
                    .finish(),
            }
        }
    }
    impl core::fmt::Display for ErrorResult {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl std::error::Error for ErrorResult {}
    #[allow(clippy::all)]
    pub fn drop_fields(fields: Fields) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "drop-fields")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_drop-fields")]
                fn wit_import(_: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(fields));
        }
    }
    #[allow(clippy::all)]
    pub fn new_fields(entries: &[(&str, &str)]) -> Fields {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            let vec3 = entries;
            let len3 = vec3.len() as i32;
            let layout3 = alloc::Layout::from_size_align_unchecked(vec3.len() * 16, 4);
            let result3 = if layout3.size() != 0 {
                let ptr = alloc::alloc(layout3);
                if ptr.is_null() {
                    alloc::handle_alloc_error(layout3);
                }
                ptr
            } else {
                core::ptr::null_mut()
            };
            for (i, e) in vec3.into_iter().enumerate() {
                let base = result3 as i32 + (i as i32) * 16;
                {
                    let (t0_0, t0_1) = e;
                    let vec1 = t0_0;
                    let ptr1 = vec1.as_ptr() as i32;
                    let len1 = vec1.len() as i32;
                    *((base + 4) as *mut i32) = len1;
                    *((base + 0) as *mut i32) = ptr1;
                    let vec2 = t0_1;
                    let ptr2 = vec2.as_ptr() as i32;
                    let len2 = vec2.len() as i32;
                    *((base + 12) as *mut i32) = len2;
                    *((base + 8) as *mut i32) = ptr2;
                }
            }

            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "new-fields")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_new-fields")]
                fn wit_import(_: i32, _: i32) -> i32;
            }
            let ret = wit_import(result3 as i32, len3);
            if layout3.size() != 0 {
                alloc::dealloc(result3, layout3);
            }
            ret as u32
        }
    }
    #[allow(clippy::all)]
    pub fn fields_get(
        fields: Fields,
        name: &str,
    ) -> wit_bindgen::rt::vec::Vec<wit_bindgen::rt::string::String> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let vec0 = name;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;
            let ptr1 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "fields-get")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_fields-get")]
                fn wit_import(_: i32, _: i32, _: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(fields), ptr0, len0, ptr1);
            let base3 = *((ptr1 + 0) as *const i32);
            let len3 = *((ptr1 + 4) as *const i32);
            let mut result3 = Vec::with_capacity(len3 as usize);
            for i in 0..len3 {
                let base = base3 + i * 8;
                result3.push({
                    let len2 = *((base + 4) as *const i32) as usize;

                    String::from_utf8(Vec::from_raw_parts(
                        *((base + 0) as *const i32) as *mut _,
                        len2,
                        len2,
                    ))
                    .unwrap()
                });
            }
            wit_bindgen::rt::dealloc(base3, (len3 as usize) * 8, 4);
            result3
        }
    }
    #[allow(clippy::all)]
    pub fn fields_set(fields: Fields, name: &str, value: &[&str]) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            let vec0 = name;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;
            let vec2 = value;
            let len2 = vec2.len() as i32;
            let layout2 = alloc::Layout::from_size_align_unchecked(vec2.len() * 8, 4);
            let result2 = if layout2.size() != 0 {
                let ptr = alloc::alloc(layout2);
                if ptr.is_null() {
                    alloc::handle_alloc_error(layout2);
                }
                ptr
            } else {
                core::ptr::null_mut()
            };
            for (i, e) in vec2.into_iter().enumerate() {
                let base = result2 as i32 + (i as i32) * 8;
                {
                    let vec1 = e;
                    let ptr1 = vec1.as_ptr() as i32;
                    let len1 = vec1.len() as i32;
                    *((base + 4) as *mut i32) = len1;
                    *((base + 0) as *mut i32) = ptr1;
                }
            }

            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "fields-set")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_fields-set")]
                fn wit_import(_: i32, _: i32, _: i32, _: i32, _: i32);
            }
            wit_import(
                wit_bindgen::rt::as_i32(fields),
                ptr0,
                len0,
                result2 as i32,
                len2,
            );
            if layout2.size() != 0 {
                alloc::dealloc(result2, layout2);
            }
        }
    }
    #[allow(clippy::all)]
    pub fn fields_delete(fields: Fields, name: &str) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            let vec0 = name;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;

            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "fields-delete")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_fields-delete")]
                fn wit_import(_: i32, _: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(fields), ptr0, len0);
        }
    }
    #[allow(clippy::all)]
    pub fn fields_append(fields: Fields, name: &str, value: &str) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            let vec0 = name;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;
            let vec1 = value;
            let ptr1 = vec1.as_ptr() as i32;
            let len1 = vec1.len() as i32;

            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "fields-append")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_fields-append")]
                fn wit_import(_: i32, _: i32, _: i32, _: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(fields), ptr0, len0, ptr1, len1);
        }
    }
    #[allow(clippy::all)]
    pub fn fields_entries(
        fields: Fields,
    ) -> wit_bindgen::rt::vec::Vec<(
        wit_bindgen::rt::string::String,
        wit_bindgen::rt::string::String,
    )> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "fields-entries")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_fields-entries")]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(fields), ptr0);
            let base3 = *((ptr0 + 0) as *const i32);
            let len3 = *((ptr0 + 4) as *const i32);
            let mut result3 = Vec::with_capacity(len3 as usize);
            for i in 0..len3 {
                let base = base3 + i * 16;
                result3.push({
                    let len1 = *((base + 4) as *const i32) as usize;
                    let len2 = *((base + 12) as *const i32) as usize;

                    (
                        String::from_utf8(Vec::from_raw_parts(
                            *((base + 0) as *const i32) as *mut _,
                            len1,
                            len1,
                        ))
                        .unwrap(),
                        String::from_utf8(Vec::from_raw_parts(
                            *((base + 8) as *const i32) as *mut _,
                            len2,
                            len2,
                        ))
                        .unwrap(),
                    )
                });
            }
            wit_bindgen::rt::dealloc(base3, (len3 as usize) * 16, 4);
            result3
        }
    }
    #[allow(clippy::all)]
    pub fn fields_clone(fields: Fields) -> Fields {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "fields-clone")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_fields-clone")]
                fn wit_import(_: i32) -> i32;
            }
            let ret = wit_import(wit_bindgen::rt::as_i32(fields));
            ret as u32
        }
    }
    #[allow(clippy::all)]
    pub fn finish_incoming_stream(s: IncomingStream) -> Option<Trailers> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "finish-incoming-stream")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_finish-incoming-stream"
                )]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(s), ptr0);
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => None,
                1 => Some(*((ptr0 + 4) as *const i32) as u32),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    pub fn finish_outgoing_stream(s: OutgoingStream, trailers: Option<Trailers>) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            let (result0_0, result0_1) = match trailers {
                Some(e) => (1i32, wit_bindgen::rt::as_i32(e)),
                None => (0i32, 0i32),
            };
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "finish-outgoing-stream")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_finish-outgoing-stream"
                )]
                fn wit_import(_: i32, _: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(s), result0_0, result0_1);
        }
    }
    #[allow(clippy::all)]
    pub fn drop_incoming_request(request: IncomingRequest) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "drop-incoming-request")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_drop-incoming-request")]
                fn wit_import(_: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(request));
        }
    }
    #[allow(clippy::all)]
    pub fn drop_outgoing_request(request: OutgoingRequest) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "drop-outgoing-request")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_drop-outgoing-request")]
                fn wit_import(_: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(request));
        }
    }
    #[allow(clippy::all)]
    pub fn incoming_request_method(request: IncomingRequest) -> MethodResult {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 12]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "incoming-request-method")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_incoming-request-method"
                )]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(request), ptr0);
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => MethodResult::Get,
                1 => MethodResult::Head,
                2 => MethodResult::Post,
                3 => MethodResult::Put,
                4 => MethodResult::Delete,
                5 => MethodResult::Connect,
                6 => MethodResult::Options,
                7 => MethodResult::Trace,
                8 => MethodResult::Patch,
                9 => MethodResult::Other({
                    let len1 = *((ptr0 + 8) as *const i32) as usize;

                    String::from_utf8(Vec::from_raw_parts(
                        *((ptr0 + 4) as *const i32) as *mut _,
                        len1,
                        len1,
                    ))
                    .unwrap()
                }),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    pub fn incoming_request_path(request: IncomingRequest) -> wit_bindgen::rt::string::String {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "incoming-request-path")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_incoming-request-path")]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(request), ptr0);
            let len1 = *((ptr0 + 4) as *const i32) as usize;
            String::from_utf8(Vec::from_raw_parts(
                *((ptr0 + 0) as *const i32) as *mut _,
                len1,
                len1,
            ))
            .unwrap()
        }
    }
    #[allow(clippy::all)]
    pub fn incoming_request_query(request: IncomingRequest) -> wit_bindgen::rt::string::String {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "incoming-request-query")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_incoming-request-query"
                )]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(request), ptr0);
            let len1 = *((ptr0 + 4) as *const i32) as usize;
            String::from_utf8(Vec::from_raw_parts(
                *((ptr0 + 0) as *const i32) as *mut _,
                len1,
                len1,
            ))
            .unwrap()
        }
    }
    #[allow(clippy::all)]
    pub fn incoming_request_scheme(request: IncomingRequest) -> Option<SchemeResult> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 16]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "incoming-request-scheme")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_incoming-request-scheme"
                )]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(request), ptr0);
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => None,
                1 => Some(match i32::from(*((ptr0 + 4) as *const u8)) {
                    0 => SchemeResult::Http,
                    1 => SchemeResult::Https,
                    2 => SchemeResult::Other({
                        let len1 = *((ptr0 + 12) as *const i32) as usize;

                        String::from_utf8(Vec::from_raw_parts(
                            *((ptr0 + 8) as *const i32) as *mut _,
                            len1,
                            len1,
                        ))
                        .unwrap()
                    }),
                    _ => panic!("invalid enum discriminant"),
                }),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    pub fn incoming_request_authority(request: IncomingRequest) -> wit_bindgen::rt::string::String {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "incoming-request-authority")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_incoming-request-authority"
                )]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(request), ptr0);
            let len1 = *((ptr0 + 4) as *const i32) as usize;
            String::from_utf8(Vec::from_raw_parts(
                *((ptr0 + 0) as *const i32) as *mut _,
                len1,
                len1,
            ))
            .unwrap()
        }
    }
    #[allow(clippy::all)]
    pub fn incoming_request_headers(request: IncomingRequest) -> Headers {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "incoming-request-headers")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_incoming-request-headers"
                )]
                fn wit_import(_: i32) -> i32;
            }
            let ret = wit_import(wit_bindgen::rt::as_i32(request));
            ret as u32
        }
    }
    #[allow(clippy::all)]
    pub fn incoming_request_consume(request: IncomingRequest) -> Result<IncomingStream, ()> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "incoming-request-consume")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_incoming-request-consume"
                )]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(request), ptr0);
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => Ok(*((ptr0 + 4) as *const i32) as u32),
                1 => Err(()),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    pub fn new_outgoing_request(
        method: MethodParam<'_>,
        path: &str,
        query: &str,
        scheme: Option<SchemeParam<'_>>,
        authority: &str,
        headers: Headers,
    ) -> OutgoingRequest {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            let (result1_0, result1_1, result1_2) = match method {
                MethodParam::Get => (0i32, 0i32, 0i32),
                MethodParam::Head => (1i32, 0i32, 0i32),
                MethodParam::Post => (2i32, 0i32, 0i32),
                MethodParam::Put => (3i32, 0i32, 0i32),
                MethodParam::Delete => (4i32, 0i32, 0i32),
                MethodParam::Connect => (5i32, 0i32, 0i32),
                MethodParam::Options => (6i32, 0i32, 0i32),
                MethodParam::Trace => (7i32, 0i32, 0i32),
                MethodParam::Patch => (8i32, 0i32, 0i32),
                MethodParam::Other(e) => {
                    let vec0 = e;
                    let ptr0 = vec0.as_ptr() as i32;
                    let len0 = vec0.len() as i32;

                    (9i32, ptr0, len0)
                }
            };
            let vec2 = path;
            let ptr2 = vec2.as_ptr() as i32;
            let len2 = vec2.len() as i32;
            let vec3 = query;
            let ptr3 = vec3.as_ptr() as i32;
            let len3 = vec3.len() as i32;
            let (result6_0, result6_1, result6_2, result6_3) = match scheme {
                Some(e) => {
                    let (result5_0, result5_1, result5_2) = match e {
                        SchemeParam::Http => (0i32, 0i32, 0i32),
                        SchemeParam::Https => (1i32, 0i32, 0i32),
                        SchemeParam::Other(e) => {
                            let vec4 = e;
                            let ptr4 = vec4.as_ptr() as i32;
                            let len4 = vec4.len() as i32;

                            (2i32, ptr4, len4)
                        }
                    };

                    (1i32, result5_0, result5_1, result5_2)
                }
                None => (0i32, 0i32, 0i32, 0i32),
            };
            let vec7 = authority;
            let ptr7 = vec7.as_ptr() as i32;
            let len7 = vec7.len() as i32;

            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "new-outgoing-request")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_new-outgoing-request")]
                fn wit_import(
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                ) -> i32;
            }
            let ret = wit_import(
                result1_0,
                result1_1,
                result1_2,
                ptr2,
                len2,
                ptr3,
                len3,
                result6_0,
                result6_1,
                result6_2,
                result6_3,
                ptr7,
                len7,
                wit_bindgen::rt::as_i32(headers),
            );
            ret as u32
        }
    }
    #[allow(clippy::all)]
    pub fn outgoing_request_write(request: OutgoingRequest) -> Result<OutgoingStream, ()> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "outgoing-request-write")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_outgoing-request-write"
                )]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(request), ptr0);
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => Ok(*((ptr0 + 4) as *const i32) as u32),
                1 => Err(()),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    pub fn drop_response_outparam(response: ResponseOutparam) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "drop-response-outparam")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_drop-response-outparam"
                )]
                fn wit_import(_: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(response));
        }
    }
    #[allow(clippy::all)]
    pub fn set_response_outparam(
        response: Result<OutgoingResponse, ErrorParam<'_>>,
    ) -> Result<(), ()> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            let (result5_0, result5_1, result5_2, result5_3) = match response {
                Ok(e) => (0i32, wit_bindgen::rt::as_i32(e), 0i32, 0i32),
                Err(e) => {
                    let (result4_0, result4_1, result4_2) = match e {
                        ErrorParam::InvalidUrl(e) => {
                            let vec0 = e;
                            let ptr0 = vec0.as_ptr() as i32;
                            let len0 = vec0.len() as i32;

                            (0i32, ptr0, len0)
                        }
                        ErrorParam::TimeoutError(e) => {
                            let vec1 = e;
                            let ptr1 = vec1.as_ptr() as i32;
                            let len1 = vec1.len() as i32;

                            (1i32, ptr1, len1)
                        }
                        ErrorParam::ProtocolError(e) => {
                            let vec2 = e;
                            let ptr2 = vec2.as_ptr() as i32;
                            let len2 = vec2.len() as i32;

                            (2i32, ptr2, len2)
                        }
                        ErrorParam::UnexpectedError(e) => {
                            let vec3 = e;
                            let ptr3 = vec3.as_ptr() as i32;
                            let len3 = vec3.len() as i32;

                            (3i32, ptr3, len3)
                        }
                    };

                    (1i32, result4_0, result4_1, result4_2)
                }
            };
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "set-response-outparam")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_set-response-outparam")]
                fn wit_import(_: i32, _: i32, _: i32, _: i32) -> i32;
            }
            let ret = wit_import(result5_0, result5_1, result5_2, result5_3);
            match ret {
                0 => Ok(()),
                1 => Err(()),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    pub fn drop_incoming_response(response: IncomingResponse) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "drop-incoming-response")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_drop-incoming-response"
                )]
                fn wit_import(_: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(response));
        }
    }
    #[allow(clippy::all)]
    pub fn drop_outgoing_response(response: OutgoingResponse) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "drop-outgoing-response")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_drop-outgoing-response"
                )]
                fn wit_import(_: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(response));
        }
    }
    #[allow(clippy::all)]
    pub fn incoming_response_status(response: IncomingResponse) -> StatusCode {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "incoming-response-status")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_incoming-response-status"
                )]
                fn wit_import(_: i32) -> i32;
            }
            let ret = wit_import(wit_bindgen::rt::as_i32(response));
            ret as u16
        }
    }
    #[allow(clippy::all)]
    pub fn incoming_response_headers(response: IncomingResponse) -> Headers {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "incoming-response-headers")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_incoming-response-headers"
                )]
                fn wit_import(_: i32) -> i32;
            }
            let ret = wit_import(wit_bindgen::rt::as_i32(response));
            ret as u32
        }
    }
    #[allow(clippy::all)]
    pub fn incoming_response_consume(response: IncomingResponse) -> Result<IncomingStream, ()> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "incoming-response-consume")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_incoming-response-consume"
                )]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(response), ptr0);
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => Ok(*((ptr0 + 4) as *const i32) as u32),
                1 => Err(()),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    pub fn new_outgoing_response(status_code: StatusCode, headers: Headers) -> OutgoingResponse {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "new-outgoing-response")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "types_new-outgoing-response")]
                fn wit_import(_: i32, _: i32) -> i32;
            }
            let ret = wit_import(
                wit_bindgen::rt::as_i32(status_code),
                wit_bindgen::rt::as_i32(headers),
            );
            ret as u32
        }
    }
    #[allow(clippy::all)]
    pub fn outgoing_response_write(response: OutgoingResponse) -> Result<OutgoingStream, ()> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "outgoing-response-write")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_outgoing-response-write"
                )]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(response), ptr0);
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => Ok(*((ptr0 + 4) as *const i32) as u32),
                1 => Err(()),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    pub fn drop_future_incoming_response(f: FutureIncomingResponse) -> () {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "drop-future-incoming-response")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_drop-future-incoming-response"
                )]
                fn wit_import(_: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(f));
        }
    }
    #[allow(clippy::all)]
    pub fn future_incoming_response_get(
        f: FutureIncomingResponse,
    ) -> Option<Result<IncomingResponse, ErrorResult>> {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 20]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "future-incoming-response-get")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_future-incoming-response-get"
                )]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen::rt::as_i32(f), ptr0);
            match i32::from(*((ptr0 + 0) as *const u8)) {
                0 => None,
                1 => Some(match i32::from(*((ptr0 + 4) as *const u8)) {
                    0 => Ok(*((ptr0 + 8) as *const i32) as u32),
                    1 => Err(match i32::from(*((ptr0 + 8) as *const u8)) {
                        0 => ErrorResult::InvalidUrl({
                            let len1 = *((ptr0 + 16) as *const i32) as usize;

                            String::from_utf8(Vec::from_raw_parts(
                                *((ptr0 + 12) as *const i32) as *mut _,
                                len1,
                                len1,
                            ))
                            .unwrap()
                        }),
                        1 => ErrorResult::TimeoutError({
                            let len2 = *((ptr0 + 16) as *const i32) as usize;

                            String::from_utf8(Vec::from_raw_parts(
                                *((ptr0 + 12) as *const i32) as *mut _,
                                len2,
                                len2,
                            ))
                            .unwrap()
                        }),
                        2 => ErrorResult::ProtocolError({
                            let len3 = *((ptr0 + 16) as *const i32) as usize;

                            String::from_utf8(Vec::from_raw_parts(
                                *((ptr0 + 12) as *const i32) as *mut _,
                                len3,
                                len3,
                            ))
                            .unwrap()
                        }),
                        3 => ErrorResult::UnexpectedError({
                            let len4 = *((ptr0 + 16) as *const i32) as usize;

                            String::from_utf8(Vec::from_raw_parts(
                                *((ptr0 + 12) as *const i32) as *mut _,
                                len4,
                                len4,
                            ))
                            .unwrap()
                        }),
                        _ => panic!("invalid enum discriminant"),
                    }),
                    _ => panic!("invalid enum discriminant"),
                }),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    #[allow(clippy::all)]
    pub fn listen_to_future_incoming_response(f: FutureIncomingResponse) -> Pollable {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            #[link(wasm_import_module = "types")]
            extern "C" {
                #[cfg_attr(
                    target_arch = "wasm32",
                    link_name = "listen-to-future-incoming-response"
                )]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "types_listen-to-future-incoming-response"
                )]
                fn wit_import(_: i32) -> i32;
            }
            let ret = wit_import(wit_bindgen::rt::as_i32(f));
            ret as u32
        }
    }
}

#[allow(clippy::all)]
pub mod default_outgoing_http {
    pub type OutgoingRequest = super::types::OutgoingRequest;
    pub type RequestOptions = super::types::RequestOptions;
    pub type FutureIncomingResponse = super::types::FutureIncomingResponse;
    #[allow(clippy::all)]
    pub fn handle(
        request: OutgoingRequest,
        options: Option<RequestOptions>,
    ) -> FutureIncomingResponse {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        unsafe {
            let (result4_0, result4_1, result4_2, result4_3, result4_4, result4_5, result4_6) =
                match options {
                    Some(e) => {
                        let RequestOptions {
                            connect_timeout_ms: connect_timeout_ms0,
                            first_byte_timeout_ms: first_byte_timeout_ms0,
                            between_bytes_timeout_ms: between_bytes_timeout_ms0,
                        } = e;
                        let (result1_0, result1_1) = match connect_timeout_ms0 {
                            Some(e) => (1i32, wit_bindgen::rt::as_i32(e)),
                            None => (0i32, 0i32),
                        };
                        let (result2_0, result2_1) = match first_byte_timeout_ms0 {
                            Some(e) => (1i32, wit_bindgen::rt::as_i32(e)),
                            None => (0i32, 0i32),
                        };
                        let (result3_0, result3_1) = match between_bytes_timeout_ms0 {
                            Some(e) => (1i32, wit_bindgen::rt::as_i32(e)),
                            None => (0i32, 0i32),
                        };
                        (
                            1i32, result1_0, result1_1, result2_0, result2_1, result3_0, result3_1,
                        )
                    }
                    None => (0i32, 0i32, 0i32, 0i32, 0i32, 0i32, 0i32),
                };
            #[link(wasm_import_module = "default-outgoing-HTTP")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "handle")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "default-outgoing-HTTP_handle"
                )]
                fn wit_import(
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                    _: i32,
                ) -> i32;
            }
            let ret = wit_import(
                wit_bindgen::rt::as_i32(request),
                result4_0,
                result4_1,
                result4_2,
                result4_3,
                result4_4,
                result4_5,
                result4_6,
            );
            ret as u32
        }
    }
}

#[allow(clippy::all)]
pub mod http {
    pub type IncomingRequest = super::types::IncomingRequest;
    pub type ResponseOutparam = super::types::ResponseOutparam;
    pub trait Http {
        fn handle(request: IncomingRequest, response_out: ResponseOutparam) -> ();
    }

    #[doc(hidden)]
    pub unsafe fn call_handle<T: Http>(arg0: i32, arg1: i32) {
        #[allow(unused_imports)]
        use wit_bindgen::rt::{alloc, string::String, vec::Vec};
        T::handle(arg0 as u32, arg1 as u32);
    }
}

/// Declares the export of the component's world for the
/// given type.

macro_rules! export_proxy(($t:ident) => {
          const _: () = {

            #[doc(hidden)]
            #[export_name = "HTTP#handle"]
            #[allow(non_snake_case)]
            unsafe extern "C" fn __export_http_handle(arg0: i32,arg1: i32,) {
              http::call_handle::<$t>(arg0,arg1,)
            }

          };

          #[used]
          #[doc(hidden)]
          #[cfg(target_arch = "wasm32")]
          static __FORCE_SECTION_REF: fn() = __force_section_ref;
          #[doc(hidden)]
          #[cfg(target_arch = "wasm32")]
          fn __force_section_ref() {
            __link_section()
          }
        });

#[cfg(target_arch = "wasm32")]
#[link_section = "component-type:proxy"]
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 8155] = [
    2, 0, 3, 119, 105, 116, 5, 112, 114, 111, 120, 121, 5, 112, 114, 111, 120, 121, 0, 97, 115,
    109, 12, 0, 1, 0, 7, 236, 17, 1, 65, 9, 1, 66, 2, 1, 121, 4, 8, 112, 111, 108, 108, 97, 98,
    108, 101, 0, 3, 0, 0, 3, 4, 112, 111, 108, 108, 20, 112, 97, 116, 104, 58, 47, 112, 111, 108,
    108, 47, 112, 111, 108, 108, 47, 112, 111, 108, 108, 5, 0, 2, 3, 0, 0, 8, 112, 111, 108, 108,
    97, 98, 108, 101, 1, 66, 8, 2, 3, 2, 1, 1, 4, 8, 112, 111, 108, 108, 97, 98, 108, 101, 0, 3, 0,
    0, 1, 114, 0, 4, 12, 115, 116, 114, 101, 97, 109, 45, 101, 114, 114, 111, 114, 0, 3, 0, 2, 1,
    121, 4, 13, 111, 117, 116, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 4, 1, 121,
    4, 12, 105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 6, 3, 7, 115, 116,
    114, 101, 97, 109, 115, 24, 112, 97, 116, 104, 58, 47, 105, 111, 47, 115, 116, 114, 101, 97,
    109, 115, 47, 115, 116, 114, 101, 97, 109, 115, 5, 2, 2, 3, 0, 1, 12, 105, 110, 112, 117, 116,
    45, 115, 116, 114, 101, 97, 109, 2, 3, 0, 1, 13, 111, 117, 116, 112, 117, 116, 45, 115, 116,
    114, 101, 97, 109, 1, 66, 110, 2, 3, 2, 1, 3, 4, 12, 105, 110, 112, 117, 116, 45, 115, 116,
    114, 101, 97, 109, 0, 3, 0, 0, 2, 3, 2, 1, 4, 4, 13, 111, 117, 116, 112, 117, 116, 45, 115,
    116, 114, 101, 97, 109, 0, 3, 0, 2, 2, 3, 2, 1, 1, 4, 8, 112, 111, 108, 108, 97, 98, 108, 101,
    0, 3, 0, 4, 1, 123, 4, 11, 115, 116, 97, 116, 117, 115, 45, 99, 111, 100, 101, 0, 3, 0, 6, 1,
    113, 3, 4, 72, 84, 84, 80, 0, 0, 5, 72, 84, 84, 80, 83, 0, 0, 5, 111, 116, 104, 101, 114, 1,
    115, 0, 4, 6, 115, 99, 104, 101, 109, 101, 0, 3, 0, 8, 1, 121, 4, 17, 114, 101, 115, 112, 111,
    110, 115, 101, 45, 111, 117, 116, 112, 97, 114, 97, 109, 0, 3, 0, 10, 1, 107, 121, 1, 114, 3,
    18, 99, 111, 110, 110, 101, 99, 116, 45, 116, 105, 109, 101, 111, 117, 116, 45, 109, 115, 12,
    21, 102, 105, 114, 115, 116, 45, 98, 121, 116, 101, 45, 116, 105, 109, 101, 111, 117, 116, 45,
    109, 115, 12, 24, 98, 101, 116, 119, 101, 101, 110, 45, 98, 121, 116, 101, 115, 45, 116, 105,
    109, 101, 111, 117, 116, 45, 109, 115, 12, 4, 15, 114, 101, 113, 117, 101, 115, 116, 45, 111,
    112, 116, 105, 111, 110, 115, 0, 3, 0, 13, 4, 15, 111, 117, 116, 103, 111, 105, 110, 103, 45,
    115, 116, 114, 101, 97, 109, 0, 3, 0, 3, 1, 121, 4, 17, 111, 117, 116, 103, 111, 105, 110, 103,
    45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 3, 0, 16, 1, 121, 4, 16, 111, 117, 116, 103,
    111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 0, 3, 0, 18, 1, 113, 10, 3, 103,
    101, 116, 0, 0, 4, 104, 101, 97, 100, 0, 0, 4, 112, 111, 115, 116, 0, 0, 3, 112, 117, 116, 0,
    0, 6, 100, 101, 108, 101, 116, 101, 0, 0, 7, 99, 111, 110, 110, 101, 99, 116, 0, 0, 7, 111,
    112, 116, 105, 111, 110, 115, 0, 0, 5, 116, 114, 97, 99, 101, 0, 0, 5, 112, 97, 116, 99, 104,
    0, 0, 5, 111, 116, 104, 101, 114, 1, 115, 0, 4, 6, 109, 101, 116, 104, 111, 100, 0, 3, 0, 20,
    4, 15, 105, 110, 99, 111, 109, 105, 110, 103, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 1, 1,
    121, 4, 17, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101,
    0, 3, 0, 23, 1, 121, 4, 16, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101,
    115, 116, 0, 3, 0, 25, 1, 121, 4, 24, 102, 117, 116, 117, 114, 101, 45, 105, 110, 99, 111, 109,
    105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 3, 0, 27, 1, 121, 4, 6, 102, 105,
    101, 108, 100, 115, 0, 3, 0, 29, 4, 8, 116, 114, 97, 105, 108, 101, 114, 115, 0, 3, 0, 30, 4,
    7, 104, 101, 97, 100, 101, 114, 115, 0, 3, 0, 30, 1, 113, 4, 11, 105, 110, 118, 97, 108, 105,
    100, 45, 117, 114, 108, 1, 115, 0, 13, 116, 105, 109, 101, 111, 117, 116, 45, 101, 114, 114,
    111, 114, 1, 115, 0, 14, 112, 114, 111, 116, 111, 99, 111, 108, 45, 101, 114, 114, 111, 114, 1,
    115, 0, 16, 117, 110, 101, 120, 112, 101, 99, 116, 101, 100, 45, 101, 114, 114, 111, 114, 1,
    115, 0, 4, 5, 101, 114, 114, 111, 114, 0, 3, 0, 33, 1, 64, 1, 6, 102, 105, 101, 108, 100, 115,
    30, 1, 0, 4, 11, 100, 114, 111, 112, 45, 102, 105, 101, 108, 100, 115, 0, 1, 35, 1, 111, 2,
    115, 115, 1, 112, 36, 1, 64, 1, 7, 101, 110, 116, 114, 105, 101, 115, 37, 0, 30, 4, 10, 110,
    101, 119, 45, 102, 105, 101, 108, 100, 115, 0, 1, 38, 1, 112, 115, 1, 64, 2, 6, 102, 105, 101,
    108, 100, 115, 30, 4, 110, 97, 109, 101, 115, 0, 39, 4, 10, 102, 105, 101, 108, 100, 115, 45,
    103, 101, 116, 0, 1, 40, 1, 64, 3, 6, 102, 105, 101, 108, 100, 115, 30, 4, 110, 97, 109, 101,
    115, 5, 118, 97, 108, 117, 101, 39, 1, 0, 4, 10, 102, 105, 101, 108, 100, 115, 45, 115, 101,
    116, 0, 1, 41, 1, 64, 2, 6, 102, 105, 101, 108, 100, 115, 30, 4, 110, 97, 109, 101, 115, 1, 0,
    4, 13, 102, 105, 101, 108, 100, 115, 45, 100, 101, 108, 101, 116, 101, 0, 1, 42, 1, 64, 3, 6,
    102, 105, 101, 108, 100, 115, 30, 4, 110, 97, 109, 101, 115, 5, 118, 97, 108, 117, 101, 115, 1,
    0, 4, 13, 102, 105, 101, 108, 100, 115, 45, 97, 112, 112, 101, 110, 100, 0, 1, 43, 1, 64, 1, 6,
    102, 105, 101, 108, 100, 115, 30, 0, 37, 4, 14, 102, 105, 101, 108, 100, 115, 45, 101, 110,
    116, 114, 105, 101, 115, 0, 1, 44, 1, 64, 1, 6, 102, 105, 101, 108, 100, 115, 30, 0, 30, 4, 12,
    102, 105, 101, 108, 100, 115, 45, 99, 108, 111, 110, 101, 0, 1, 45, 1, 107, 31, 1, 64, 1, 1,
    115, 22, 0, 46, 4, 22, 102, 105, 110, 105, 115, 104, 45, 105, 110, 99, 111, 109, 105, 110, 103,
    45, 115, 116, 114, 101, 97, 109, 0, 1, 47, 1, 64, 2, 1, 115, 15, 8, 116, 114, 97, 105, 108,
    101, 114, 115, 46, 1, 0, 4, 22, 102, 105, 110, 105, 115, 104, 45, 111, 117, 116, 103, 111, 105,
    110, 103, 45, 115, 116, 114, 101, 97, 109, 0, 1, 48, 1, 64, 1, 7, 114, 101, 113, 117, 101, 115,
    116, 26, 1, 0, 4, 21, 100, 114, 111, 112, 45, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114,
    101, 113, 117, 101, 115, 116, 0, 1, 49, 1, 64, 1, 7, 114, 101, 113, 117, 101, 115, 116, 19, 1,
    0, 4, 21, 100, 114, 111, 112, 45, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113,
    117, 101, 115, 116, 0, 1, 50, 1, 64, 1, 7, 114, 101, 113, 117, 101, 115, 116, 26, 0, 21, 4, 23,
    105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 45, 109, 101,
    116, 104, 111, 100, 0, 1, 51, 1, 64, 1, 7, 114, 101, 113, 117, 101, 115, 116, 26, 0, 115, 4,
    21, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 45, 112, 97,
    116, 104, 0, 1, 52, 4, 22, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101,
    115, 116, 45, 113, 117, 101, 114, 121, 0, 1, 52, 1, 107, 9, 1, 64, 1, 7, 114, 101, 113, 117,
    101, 115, 116, 26, 0, 53, 4, 23, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117,
    101, 115, 116, 45, 115, 99, 104, 101, 109, 101, 0, 1, 54, 4, 26, 105, 110, 99, 111, 109, 105,
    110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 45, 97, 117, 116, 104, 111, 114, 105, 116,
    121, 0, 1, 52, 1, 64, 1, 7, 114, 101, 113, 117, 101, 115, 116, 26, 0, 32, 4, 24, 105, 110, 99,
    111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 45, 104, 101, 97, 100, 101,
    114, 115, 0, 1, 55, 1, 106, 1, 22, 0, 1, 64, 1, 7, 114, 101, 113, 117, 101, 115, 116, 26, 0,
    56, 4, 24, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 45,
    99, 111, 110, 115, 117, 109, 101, 0, 1, 57, 1, 64, 6, 6, 109, 101, 116, 104, 111, 100, 21, 4,
    112, 97, 116, 104, 115, 5, 113, 117, 101, 114, 121, 115, 6, 115, 99, 104, 101, 109, 101, 53, 9,
    97, 117, 116, 104, 111, 114, 105, 116, 121, 115, 7, 104, 101, 97, 100, 101, 114, 115, 32, 0,
    19, 4, 20, 110, 101, 119, 45, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117,
    101, 115, 116, 0, 1, 58, 1, 106, 1, 15, 0, 1, 64, 1, 7, 114, 101, 113, 117, 101, 115, 116, 19,
    0, 59, 4, 22, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116,
    45, 119, 114, 105, 116, 101, 0, 1, 60, 1, 64, 1, 8, 114, 101, 115, 112, 111, 110, 115, 101, 11,
    1, 0, 4, 22, 100, 114, 111, 112, 45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 111, 117, 116,
    112, 97, 114, 97, 109, 0, 1, 61, 1, 106, 1, 17, 1, 34, 1, 106, 0, 0, 1, 64, 1, 8, 114, 101,
    115, 112, 111, 110, 115, 101, 62, 0, 63, 4, 21, 115, 101, 116, 45, 114, 101, 115, 112, 111,
    110, 115, 101, 45, 111, 117, 116, 112, 97, 114, 97, 109, 0, 1, 64, 1, 64, 1, 8, 114, 101, 115,
    112, 111, 110, 115, 101, 24, 1, 0, 4, 22, 100, 114, 111, 112, 45, 105, 110, 99, 111, 109, 105,
    110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 1, 65, 1, 64, 1, 8, 114, 101, 115,
    112, 111, 110, 115, 101, 17, 1, 0, 4, 22, 100, 114, 111, 112, 45, 111, 117, 116, 103, 111, 105,
    110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 1, 66, 1, 64, 1, 8, 114, 101, 115,
    112, 111, 110, 115, 101, 24, 0, 7, 4, 24, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101,
    115, 112, 111, 110, 115, 101, 45, 115, 116, 97, 116, 117, 115, 0, 1, 67, 1, 64, 1, 8, 114, 101,
    115, 112, 111, 110, 115, 101, 24, 0, 32, 4, 25, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114,
    101, 115, 112, 111, 110, 115, 101, 45, 104, 101, 97, 100, 101, 114, 115, 0, 1, 68, 1, 64, 1, 8,
    114, 101, 115, 112, 111, 110, 115, 101, 24, 0, 56, 4, 25, 105, 110, 99, 111, 109, 105, 110,
    103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 99, 111, 110, 115, 117, 109, 101, 0, 1,
    69, 1, 64, 2, 11, 115, 116, 97, 116, 117, 115, 45, 99, 111, 100, 101, 7, 7, 104, 101, 97, 100,
    101, 114, 115, 32, 0, 17, 4, 21, 110, 101, 119, 45, 111, 117, 116, 103, 111, 105, 110, 103, 45,
    114, 101, 115, 112, 111, 110, 115, 101, 0, 1, 70, 1, 64, 1, 8, 114, 101, 115, 112, 111, 110,
    115, 101, 17, 0, 59, 4, 23, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 115, 112,
    111, 110, 115, 101, 45, 119, 114, 105, 116, 101, 0, 1, 71, 1, 64, 1, 1, 102, 28, 1, 0, 4, 29,
    100, 114, 111, 112, 45, 102, 117, 116, 117, 114, 101, 45, 105, 110, 99, 111, 109, 105, 110,
    103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 1, 72, 1, 106, 1, 24, 1, 34, 1, 107, 201,
    0, 1, 64, 1, 1, 102, 28, 0, 202, 0, 4, 28, 102, 117, 116, 117, 114, 101, 45, 105, 110, 99, 111,
    109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 103, 101, 116, 0, 1, 75, 1,
    64, 1, 1, 102, 28, 0, 5, 4, 34, 108, 105, 115, 116, 101, 110, 45, 116, 111, 45, 102, 117, 116,
    117, 114, 101, 45, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110,
    115, 101, 0, 1, 76, 4, 5, 116, 121, 112, 101, 115, 16, 112, 107, 103, 58, 47, 116, 121, 112,
    101, 115, 47, 116, 121, 112, 101, 115, 5, 5, 11, 21, 1, 5, 116, 121, 112, 101, 115, 10, 112,
    107, 103, 58, 47, 116, 121, 112, 101, 115, 3, 0, 0, 7, 246, 8, 1, 65, 14, 1, 66, 2, 1, 121, 4,
    8, 112, 111, 108, 108, 97, 98, 108, 101, 0, 3, 0, 0, 3, 4, 112, 111, 108, 108, 20, 112, 97,
    116, 104, 58, 47, 112, 111, 108, 108, 47, 112, 111, 108, 108, 47, 112, 111, 108, 108, 5, 0, 2,
    3, 0, 0, 8, 112, 111, 108, 108, 97, 98, 108, 101, 1, 66, 8, 2, 3, 2, 1, 1, 4, 8, 112, 111, 108,
    108, 97, 98, 108, 101, 0, 3, 0, 0, 1, 114, 0, 4, 12, 115, 116, 114, 101, 97, 109, 45, 101, 114,
    114, 111, 114, 0, 3, 0, 2, 1, 121, 4, 13, 111, 117, 116, 112, 117, 116, 45, 115, 116, 114, 101,
    97, 109, 0, 3, 0, 4, 1, 121, 4, 12, 105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109,
    0, 3, 0, 6, 3, 7, 115, 116, 114, 101, 97, 109, 115, 24, 112, 97, 116, 104, 58, 47, 105, 111,
    47, 115, 116, 114, 101, 97, 109, 115, 47, 115, 116, 114, 101, 97, 109, 115, 5, 2, 2, 3, 0, 1,
    12, 105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 2, 3, 0, 1, 13, 111, 117, 116,
    112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 1, 66, 35, 2, 3, 2, 1, 3, 4, 12, 105, 110, 112,
    117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 0, 2, 3, 2, 1, 4, 4, 13, 111, 117, 116,
    112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 2, 2, 3, 2, 1, 1, 4, 8, 112, 111, 108,
    108, 97, 98, 108, 101, 0, 3, 0, 4, 1, 123, 4, 11, 115, 116, 97, 116, 117, 115, 45, 99, 111,
    100, 101, 0, 3, 0, 6, 1, 113, 3, 4, 72, 84, 84, 80, 0, 0, 5, 72, 84, 84, 80, 83, 0, 0, 5, 111,
    116, 104, 101, 114, 1, 115, 0, 4, 6, 115, 99, 104, 101, 109, 101, 0, 3, 0, 8, 1, 121, 4, 17,
    114, 101, 115, 112, 111, 110, 115, 101, 45, 111, 117, 116, 112, 97, 114, 97, 109, 0, 3, 0, 10,
    1, 107, 121, 1, 114, 3, 18, 99, 111, 110, 110, 101, 99, 116, 45, 116, 105, 109, 101, 111, 117,
    116, 45, 109, 115, 12, 21, 102, 105, 114, 115, 116, 45, 98, 121, 116, 101, 45, 116, 105, 109,
    101, 111, 117, 116, 45, 109, 115, 12, 24, 98, 101, 116, 119, 101, 101, 110, 45, 98, 121, 116,
    101, 115, 45, 116, 105, 109, 101, 111, 117, 116, 45, 109, 115, 12, 4, 15, 114, 101, 113, 117,
    101, 115, 116, 45, 111, 112, 116, 105, 111, 110, 115, 0, 3, 0, 13, 4, 15, 111, 117, 116, 103,
    111, 105, 110, 103, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 3, 1, 121, 4, 17, 111, 117, 116,
    103, 111, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 3, 0, 16, 1, 121, 4,
    16, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 0, 3, 0, 18,
    1, 113, 10, 3, 103, 101, 116, 0, 0, 4, 104, 101, 97, 100, 0, 0, 4, 112, 111, 115, 116, 0, 0, 3,
    112, 117, 116, 0, 0, 6, 100, 101, 108, 101, 116, 101, 0, 0, 7, 99, 111, 110, 110, 101, 99, 116,
    0, 0, 7, 111, 112, 116, 105, 111, 110, 115, 0, 0, 5, 116, 114, 97, 99, 101, 0, 0, 5, 112, 97,
    116, 99, 104, 0, 0, 5, 111, 116, 104, 101, 114, 1, 115, 0, 4, 6, 109, 101, 116, 104, 111, 100,
    0, 3, 0, 20, 4, 15, 105, 110, 99, 111, 109, 105, 110, 103, 45, 115, 116, 114, 101, 97, 109, 0,
    3, 0, 1, 1, 121, 4, 17, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111,
    110, 115, 101, 0, 3, 0, 23, 1, 121, 4, 16, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101,
    113, 117, 101, 115, 116, 0, 3, 0, 25, 1, 121, 4, 24, 102, 117, 116, 117, 114, 101, 45, 105,
    110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 3, 0, 27, 1,
    121, 4, 6, 102, 105, 101, 108, 100, 115, 0, 3, 0, 29, 4, 8, 116, 114, 97, 105, 108, 101, 114,
    115, 0, 3, 0, 30, 4, 7, 104, 101, 97, 100, 101, 114, 115, 0, 3, 0, 30, 1, 113, 4, 11, 105, 110,
    118, 97, 108, 105, 100, 45, 117, 114, 108, 1, 115, 0, 13, 116, 105, 109, 101, 111, 117, 116,
    45, 101, 114, 114, 111, 114, 1, 115, 0, 14, 112, 114, 111, 116, 111, 99, 111, 108, 45, 101,
    114, 114, 111, 114, 1, 115, 0, 16, 117, 110, 101, 120, 112, 101, 99, 116, 101, 100, 45, 101,
    114, 114, 111, 114, 1, 115, 0, 4, 5, 101, 114, 114, 111, 114, 0, 3, 0, 33, 3, 5, 116, 121, 112,
    101, 115, 16, 112, 107, 103, 58, 47, 116, 121, 112, 101, 115, 47, 116, 121, 112, 101, 115, 5,
    5, 2, 3, 0, 2, 16, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115,
    116, 2, 3, 0, 2, 15, 114, 101, 113, 117, 101, 115, 116, 45, 111, 112, 116, 105, 111, 110, 115,
    2, 3, 0, 2, 24, 102, 117, 116, 117, 114, 101, 45, 105, 110, 99, 111, 109, 105, 110, 103, 45,
    114, 101, 115, 112, 111, 110, 115, 101, 1, 66, 9, 2, 3, 2, 1, 6, 4, 16, 111, 117, 116, 103,
    111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 0, 3, 0, 0, 2, 3, 2, 1, 7, 4, 15,
    114, 101, 113, 117, 101, 115, 116, 45, 111, 112, 116, 105, 111, 110, 115, 0, 3, 0, 2, 2, 3, 2,
    1, 8, 4, 24, 102, 117, 116, 117, 114, 101, 45, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114,
    101, 115, 112, 111, 110, 115, 101, 0, 3, 0, 4, 1, 107, 3, 1, 64, 2, 7, 114, 101, 113, 117, 101,
    115, 116, 1, 7, 111, 112, 116, 105, 111, 110, 115, 6, 0, 5, 4, 6, 104, 97, 110, 100, 108, 101,
    0, 1, 7, 4, 16, 111, 117, 116, 103, 111, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114,
    38, 112, 107, 103, 58, 47, 111, 117, 116, 103, 111, 105, 110, 103, 45, 104, 97, 110, 100, 108,
    101, 114, 47, 111, 117, 116, 103, 111, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114, 5,
    9, 11, 43, 1, 16, 111, 117, 116, 103, 111, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114,
    21, 112, 107, 103, 58, 47, 111, 117, 116, 103, 111, 105, 110, 103, 45, 104, 97, 110, 100, 108,
    101, 114, 3, 2, 0, 7, 188, 8, 1, 65, 13, 1, 66, 2, 1, 121, 4, 8, 112, 111, 108, 108, 97, 98,
    108, 101, 0, 3, 0, 0, 3, 4, 112, 111, 108, 108, 20, 112, 97, 116, 104, 58, 47, 112, 111, 108,
    108, 47, 112, 111, 108, 108, 47, 112, 111, 108, 108, 5, 0, 2, 3, 0, 0, 8, 112, 111, 108, 108,
    97, 98, 108, 101, 1, 66, 8, 2, 3, 2, 1, 1, 4, 8, 112, 111, 108, 108, 97, 98, 108, 101, 0, 3, 0,
    0, 1, 114, 0, 4, 12, 115, 116, 114, 101, 97, 109, 45, 101, 114, 114, 111, 114, 0, 3, 0, 2, 1,
    121, 4, 13, 111, 117, 116, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 4, 1, 121,
    4, 12, 105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 6, 3, 7, 115, 116,
    114, 101, 97, 109, 115, 24, 112, 97, 116, 104, 58, 47, 105, 111, 47, 115, 116, 114, 101, 97,
    109, 115, 47, 115, 116, 114, 101, 97, 109, 115, 5, 2, 2, 3, 0, 1, 12, 105, 110, 112, 117, 116,
    45, 115, 116, 114, 101, 97, 109, 2, 3, 0, 1, 13, 111, 117, 116, 112, 117, 116, 45, 115, 116,
    114, 101, 97, 109, 1, 66, 35, 2, 3, 2, 1, 3, 4, 12, 105, 110, 112, 117, 116, 45, 115, 116, 114,
    101, 97, 109, 0, 3, 0, 0, 2, 3, 2, 1, 4, 4, 13, 111, 117, 116, 112, 117, 116, 45, 115, 116,
    114, 101, 97, 109, 0, 3, 0, 2, 2, 3, 2, 1, 1, 4, 8, 112, 111, 108, 108, 97, 98, 108, 101, 0, 3,
    0, 4, 1, 123, 4, 11, 115, 116, 97, 116, 117, 115, 45, 99, 111, 100, 101, 0, 3, 0, 6, 1, 113, 3,
    4, 72, 84, 84, 80, 0, 0, 5, 72, 84, 84, 80, 83, 0, 0, 5, 111, 116, 104, 101, 114, 1, 115, 0, 4,
    6, 115, 99, 104, 101, 109, 101, 0, 3, 0, 8, 1, 121, 4, 17, 114, 101, 115, 112, 111, 110, 115,
    101, 45, 111, 117, 116, 112, 97, 114, 97, 109, 0, 3, 0, 10, 1, 107, 121, 1, 114, 3, 18, 99,
    111, 110, 110, 101, 99, 116, 45, 116, 105, 109, 101, 111, 117, 116, 45, 109, 115, 12, 21, 102,
    105, 114, 115, 116, 45, 98, 121, 116, 101, 45, 116, 105, 109, 101, 111, 117, 116, 45, 109, 115,
    12, 24, 98, 101, 116, 119, 101, 101, 110, 45, 98, 121, 116, 101, 115, 45, 116, 105, 109, 101,
    111, 117, 116, 45, 109, 115, 12, 4, 15, 114, 101, 113, 117, 101, 115, 116, 45, 111, 112, 116,
    105, 111, 110, 115, 0, 3, 0, 13, 4, 15, 111, 117, 116, 103, 111, 105, 110, 103, 45, 115, 116,
    114, 101, 97, 109, 0, 3, 0, 3, 1, 121, 4, 17, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114,
    101, 115, 112, 111, 110, 115, 101, 0, 3, 0, 16, 1, 121, 4, 16, 111, 117, 116, 103, 111, 105,
    110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 0, 3, 0, 18, 1, 113, 10, 3, 103, 101, 116, 0,
    0, 4, 104, 101, 97, 100, 0, 0, 4, 112, 111, 115, 116, 0, 0, 3, 112, 117, 116, 0, 0, 6, 100,
    101, 108, 101, 116, 101, 0, 0, 7, 99, 111, 110, 110, 101, 99, 116, 0, 0, 7, 111, 112, 116, 105,
    111, 110, 115, 0, 0, 5, 116, 114, 97, 99, 101, 0, 0, 5, 112, 97, 116, 99, 104, 0, 0, 5, 111,
    116, 104, 101, 114, 1, 115, 0, 4, 6, 109, 101, 116, 104, 111, 100, 0, 3, 0, 20, 4, 15, 105,
    110, 99, 111, 109, 105, 110, 103, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 1, 1, 121, 4, 17,
    105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 3, 0, 23,
    1, 121, 4, 16, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 0,
    3, 0, 25, 1, 121, 4, 24, 102, 117, 116, 117, 114, 101, 45, 105, 110, 99, 111, 109, 105, 110,
    103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 3, 0, 27, 1, 121, 4, 6, 102, 105, 101, 108,
    100, 115, 0, 3, 0, 29, 4, 8, 116, 114, 97, 105, 108, 101, 114, 115, 0, 3, 0, 30, 4, 7, 104,
    101, 97, 100, 101, 114, 115, 0, 3, 0, 30, 1, 113, 4, 11, 105, 110, 118, 97, 108, 105, 100, 45,
    117, 114, 108, 1, 115, 0, 13, 116, 105, 109, 101, 111, 117, 116, 45, 101, 114, 114, 111, 114,
    1, 115, 0, 14, 112, 114, 111, 116, 111, 99, 111, 108, 45, 101, 114, 114, 111, 114, 1, 115, 0,
    16, 117, 110, 101, 120, 112, 101, 99, 116, 101, 100, 45, 101, 114, 114, 111, 114, 1, 115, 0, 4,
    5, 101, 114, 114, 111, 114, 0, 3, 0, 33, 3, 5, 116, 121, 112, 101, 115, 16, 112, 107, 103, 58,
    47, 116, 121, 112, 101, 115, 47, 116, 121, 112, 101, 115, 5, 5, 2, 3, 0, 2, 16, 105, 110, 99,
    111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 2, 3, 0, 2, 17, 114, 101, 115,
    112, 111, 110, 115, 101, 45, 111, 117, 116, 112, 97, 114, 97, 109, 1, 66, 6, 2, 3, 2, 1, 6, 4,
    16, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 0, 3, 0, 0,
    2, 3, 2, 1, 7, 4, 17, 114, 101, 115, 112, 111, 110, 115, 101, 45, 111, 117, 116, 112, 97, 114,
    97, 109, 0, 3, 0, 2, 1, 64, 2, 7, 114, 101, 113, 117, 101, 115, 116, 1, 12, 114, 101, 115, 112,
    111, 110, 115, 101, 45, 111, 117, 116, 3, 1, 0, 4, 6, 104, 97, 110, 100, 108, 101, 0, 1, 4, 4,
    16, 105, 110, 99, 111, 109, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114, 38, 112, 107,
    103, 58, 47, 105, 110, 99, 111, 109, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114, 47,
    105, 110, 99, 111, 109, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114, 5, 8, 11, 43, 1,
    16, 105, 110, 99, 111, 109, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114, 21, 112, 107,
    103, 58, 47, 105, 110, 99, 111, 109, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114, 3, 4,
    0, 7, 224, 26, 1, 65, 2, 1, 65, 22, 1, 66, 8, 1, 112, 125, 1, 64, 1, 3, 108, 101, 110, 119, 0,
    0, 4, 16, 103, 101, 116, 45, 114, 97, 110, 100, 111, 109, 45, 98, 121, 116, 101, 115, 0, 1, 1,
    1, 64, 0, 0, 119, 4, 14, 103, 101, 116, 45, 114, 97, 110, 100, 111, 109, 45, 117, 54, 52, 0, 1,
    2, 1, 111, 2, 119, 119, 1, 64, 0, 0, 3, 4, 15, 105, 110, 115, 101, 99, 117, 114, 101, 45, 114,
    97, 110, 100, 111, 109, 0, 1, 4, 3, 6, 114, 97, 110, 100, 111, 109, 26, 112, 97, 116, 104, 58,
    47, 114, 97, 110, 100, 111, 109, 47, 114, 97, 110, 100, 111, 109, 47, 114, 97, 110, 100, 111,
    109, 5, 0, 1, 66, 4, 1, 109, 5, 5, 116, 114, 97, 99, 101, 5, 100, 101, 98, 117, 103, 4, 105,
    110, 102, 111, 4, 119, 97, 114, 110, 5, 101, 114, 114, 111, 114, 4, 5, 108, 101, 118, 101, 108,
    0, 3, 0, 0, 1, 64, 3, 5, 108, 101, 118, 101, 108, 1, 7, 99, 111, 110, 116, 101, 120, 116, 115,
    7, 109, 101, 115, 115, 97, 103, 101, 115, 1, 0, 4, 3, 108, 111, 103, 0, 1, 2, 3, 7, 99, 111,
    110, 115, 111, 108, 101, 29, 112, 97, 116, 104, 58, 47, 108, 111, 103, 103, 105, 110, 103, 47,
    104, 97, 110, 100, 108, 101, 114, 47, 104, 97, 110, 100, 108, 101, 114, 5, 1, 1, 66, 8, 1, 121,
    4, 8, 112, 111, 108, 108, 97, 98, 108, 101, 0, 3, 0, 0, 1, 64, 1, 4, 116, 104, 105, 115, 1, 1,
    0, 4, 13, 100, 114, 111, 112, 45, 112, 111, 108, 108, 97, 98, 108, 101, 0, 1, 2, 1, 112, 1, 1,
    112, 125, 1, 64, 1, 2, 105, 110, 3, 0, 4, 4, 11, 112, 111, 108, 108, 45, 111, 110, 101, 111,
    102, 102, 0, 1, 5, 3, 4, 112, 111, 108, 108, 20, 112, 97, 116, 104, 58, 47, 112, 111, 108, 108,
    47, 112, 111, 108, 108, 47, 112, 111, 108, 108, 5, 2, 2, 3, 0, 2, 8, 112, 111, 108, 108, 97,
    98, 108, 101, 1, 66, 34, 2, 3, 2, 1, 3, 4, 8, 112, 111, 108, 108, 97, 98, 108, 101, 0, 3, 0, 0,
    1, 114, 0, 4, 12, 115, 116, 114, 101, 97, 109, 45, 101, 114, 114, 111, 114, 0, 3, 0, 2, 1, 121,
    4, 13, 111, 117, 116, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 4, 1, 121, 4,
    12, 105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 6, 1, 112, 125, 1, 111,
    2, 8, 127, 1, 106, 1, 9, 1, 3, 1, 64, 2, 4, 116, 104, 105, 115, 7, 3, 108, 101, 110, 119, 0,
    10, 4, 4, 114, 101, 97, 100, 0, 1, 11, 1, 111, 2, 119, 127, 1, 106, 1, 12, 1, 3, 1, 64, 2, 4,
    116, 104, 105, 115, 7, 3, 108, 101, 110, 119, 0, 13, 4, 4, 115, 107, 105, 112, 0, 1, 14, 1, 64,
    1, 4, 116, 104, 105, 115, 7, 0, 1, 4, 25, 115, 117, 98, 115, 99, 114, 105, 98, 101, 45, 116,
    111, 45, 105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 1, 15, 1, 64, 1, 4, 116,
    104, 105, 115, 7, 1, 0, 4, 17, 100, 114, 111, 112, 45, 105, 110, 112, 117, 116, 45, 115, 116,
    114, 101, 97, 109, 0, 1, 16, 1, 106, 1, 119, 1, 3, 1, 64, 2, 4, 116, 104, 105, 115, 5, 3, 98,
    117, 102, 8, 0, 17, 4, 5, 119, 114, 105, 116, 101, 0, 1, 18, 1, 64, 2, 4, 116, 104, 105, 115,
    5, 3, 108, 101, 110, 119, 0, 17, 4, 12, 119, 114, 105, 116, 101, 45, 122, 101, 114, 111, 101,
    115, 0, 1, 19, 1, 64, 3, 4, 116, 104, 105, 115, 5, 3, 115, 114, 99, 7, 3, 108, 101, 110, 119,
    0, 13, 4, 6, 115, 112, 108, 105, 99, 101, 0, 1, 20, 1, 64, 2, 4, 116, 104, 105, 115, 5, 3, 115,
    114, 99, 7, 0, 17, 4, 7, 102, 111, 114, 119, 97, 114, 100, 0, 1, 21, 1, 64, 1, 4, 116, 104,
    105, 115, 5, 0, 1, 4, 26, 115, 117, 98, 115, 99, 114, 105, 98, 101, 45, 116, 111, 45, 111, 117,
    116, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 1, 22, 1, 64, 1, 4, 116, 104, 105, 115,
    5, 1, 0, 4, 18, 100, 114, 111, 112, 45, 111, 117, 116, 112, 117, 116, 45, 115, 116, 114, 101,
    97, 109, 0, 1, 23, 3, 7, 115, 116, 114, 101, 97, 109, 115, 24, 112, 97, 116, 104, 58, 47, 105,
    111, 47, 115, 116, 114, 101, 97, 109, 115, 47, 115, 116, 114, 101, 97, 109, 115, 5, 4, 2, 3, 0,
    3, 12, 105, 110, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 2, 3, 0, 3, 13, 111, 117, 116,
    112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 1, 66, 110, 2, 3, 2, 1, 5, 4, 12, 105, 110,
    112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 0, 2, 3, 2, 1, 6, 4, 13, 111, 117,
    116, 112, 117, 116, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 2, 2, 3, 2, 1, 3, 4, 8, 112, 111,
    108, 108, 97, 98, 108, 101, 0, 3, 0, 4, 1, 123, 4, 11, 115, 116, 97, 116, 117, 115, 45, 99,
    111, 100, 101, 0, 3, 0, 6, 1, 113, 3, 4, 72, 84, 84, 80, 0, 0, 5, 72, 84, 84, 80, 83, 0, 0, 5,
    111, 116, 104, 101, 114, 1, 115, 0, 4, 6, 115, 99, 104, 101, 109, 101, 0, 3, 0, 8, 1, 121, 4,
    17, 114, 101, 115, 112, 111, 110, 115, 101, 45, 111, 117, 116, 112, 97, 114, 97, 109, 0, 3, 0,
    10, 1, 107, 121, 1, 114, 3, 18, 99, 111, 110, 110, 101, 99, 116, 45, 116, 105, 109, 101, 111,
    117, 116, 45, 109, 115, 12, 21, 102, 105, 114, 115, 116, 45, 98, 121, 116, 101, 45, 116, 105,
    109, 101, 111, 117, 116, 45, 109, 115, 12, 24, 98, 101, 116, 119, 101, 101, 110, 45, 98, 121,
    116, 101, 115, 45, 116, 105, 109, 101, 111, 117, 116, 45, 109, 115, 12, 4, 15, 114, 101, 113,
    117, 101, 115, 116, 45, 111, 112, 116, 105, 111, 110, 115, 0, 3, 0, 13, 4, 15, 111, 117, 116,
    103, 111, 105, 110, 103, 45, 115, 116, 114, 101, 97, 109, 0, 3, 0, 3, 1, 121, 4, 17, 111, 117,
    116, 103, 111, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 3, 0, 16, 1, 121,
    4, 16, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 0, 3, 0,
    18, 1, 113, 10, 3, 103, 101, 116, 0, 0, 4, 104, 101, 97, 100, 0, 0, 4, 112, 111, 115, 116, 0,
    0, 3, 112, 117, 116, 0, 0, 6, 100, 101, 108, 101, 116, 101, 0, 0, 7, 99, 111, 110, 110, 101,
    99, 116, 0, 0, 7, 111, 112, 116, 105, 111, 110, 115, 0, 0, 5, 116, 114, 97, 99, 101, 0, 0, 5,
    112, 97, 116, 99, 104, 0, 0, 5, 111, 116, 104, 101, 114, 1, 115, 0, 4, 6, 109, 101, 116, 104,
    111, 100, 0, 3, 0, 20, 4, 15, 105, 110, 99, 111, 109, 105, 110, 103, 45, 115, 116, 114, 101,
    97, 109, 0, 3, 0, 1, 1, 121, 4, 17, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115,
    112, 111, 110, 115, 101, 0, 3, 0, 23, 1, 121, 4, 16, 105, 110, 99, 111, 109, 105, 110, 103, 45,
    114, 101, 113, 117, 101, 115, 116, 0, 3, 0, 25, 1, 121, 4, 24, 102, 117, 116, 117, 114, 101,
    45, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 3, 0,
    27, 1, 121, 4, 6, 102, 105, 101, 108, 100, 115, 0, 3, 0, 29, 4, 8, 116, 114, 97, 105, 108, 101,
    114, 115, 0, 3, 0, 30, 4, 7, 104, 101, 97, 100, 101, 114, 115, 0, 3, 0, 30, 1, 113, 4, 11, 105,
    110, 118, 97, 108, 105, 100, 45, 117, 114, 108, 1, 115, 0, 13, 116, 105, 109, 101, 111, 117,
    116, 45, 101, 114, 114, 111, 114, 1, 115, 0, 14, 112, 114, 111, 116, 111, 99, 111, 108, 45,
    101, 114, 114, 111, 114, 1, 115, 0, 16, 117, 110, 101, 120, 112, 101, 99, 116, 101, 100, 45,
    101, 114, 114, 111, 114, 1, 115, 0, 4, 5, 101, 114, 114, 111, 114, 0, 3, 0, 33, 1, 64, 1, 6,
    102, 105, 101, 108, 100, 115, 30, 1, 0, 4, 11, 100, 114, 111, 112, 45, 102, 105, 101, 108, 100,
    115, 0, 1, 35, 1, 111, 2, 115, 115, 1, 112, 36, 1, 64, 1, 7, 101, 110, 116, 114, 105, 101, 115,
    37, 0, 30, 4, 10, 110, 101, 119, 45, 102, 105, 101, 108, 100, 115, 0, 1, 38, 1, 112, 115, 1,
    64, 2, 6, 102, 105, 101, 108, 100, 115, 30, 4, 110, 97, 109, 101, 115, 0, 39, 4, 10, 102, 105,
    101, 108, 100, 115, 45, 103, 101, 116, 0, 1, 40, 1, 64, 3, 6, 102, 105, 101, 108, 100, 115, 30,
    4, 110, 97, 109, 101, 115, 5, 118, 97, 108, 117, 101, 39, 1, 0, 4, 10, 102, 105, 101, 108, 100,
    115, 45, 115, 101, 116, 0, 1, 41, 1, 64, 2, 6, 102, 105, 101, 108, 100, 115, 30, 4, 110, 97,
    109, 101, 115, 1, 0, 4, 13, 102, 105, 101, 108, 100, 115, 45, 100, 101, 108, 101, 116, 101, 0,
    1, 42, 1, 64, 3, 6, 102, 105, 101, 108, 100, 115, 30, 4, 110, 97, 109, 101, 115, 5, 118, 97,
    108, 117, 101, 115, 1, 0, 4, 13, 102, 105, 101, 108, 100, 115, 45, 97, 112, 112, 101, 110, 100,
    0, 1, 43, 1, 64, 1, 6, 102, 105, 101, 108, 100, 115, 30, 0, 37, 4, 14, 102, 105, 101, 108, 100,
    115, 45, 101, 110, 116, 114, 105, 101, 115, 0, 1, 44, 1, 64, 1, 6, 102, 105, 101, 108, 100,
    115, 30, 0, 30, 4, 12, 102, 105, 101, 108, 100, 115, 45, 99, 108, 111, 110, 101, 0, 1, 45, 1,
    107, 31, 1, 64, 1, 1, 115, 22, 0, 46, 4, 22, 102, 105, 110, 105, 115, 104, 45, 105, 110, 99,
    111, 109, 105, 110, 103, 45, 115, 116, 114, 101, 97, 109, 0, 1, 47, 1, 64, 2, 1, 115, 15, 8,
    116, 114, 97, 105, 108, 101, 114, 115, 46, 1, 0, 4, 22, 102, 105, 110, 105, 115, 104, 45, 111,
    117, 116, 103, 111, 105, 110, 103, 45, 115, 116, 114, 101, 97, 109, 0, 1, 48, 1, 64, 1, 7, 114,
    101, 113, 117, 101, 115, 116, 26, 1, 0, 4, 21, 100, 114, 111, 112, 45, 105, 110, 99, 111, 109,
    105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 0, 1, 49, 1, 64, 1, 7, 114, 101, 113,
    117, 101, 115, 116, 19, 1, 0, 4, 21, 100, 114, 111, 112, 45, 111, 117, 116, 103, 111, 105, 110,
    103, 45, 114, 101, 113, 117, 101, 115, 116, 0, 1, 50, 1, 64, 1, 7, 114, 101, 113, 117, 101,
    115, 116, 26, 0, 21, 4, 23, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101,
    115, 116, 45, 109, 101, 116, 104, 111, 100, 0, 1, 51, 1, 64, 1, 7, 114, 101, 113, 117, 101,
    115, 116, 26, 0, 115, 4, 21, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117,
    101, 115, 116, 45, 112, 97, 116, 104, 0, 1, 52, 4, 22, 105, 110, 99, 111, 109, 105, 110, 103,
    45, 114, 101, 113, 117, 101, 115, 116, 45, 113, 117, 101, 114, 121, 0, 1, 52, 1, 107, 9, 1, 64,
    1, 7, 114, 101, 113, 117, 101, 115, 116, 26, 0, 53, 4, 23, 105, 110, 99, 111, 109, 105, 110,
    103, 45, 114, 101, 113, 117, 101, 115, 116, 45, 115, 99, 104, 101, 109, 101, 0, 1, 54, 4, 26,
    105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 45, 97, 117, 116,
    104, 111, 114, 105, 116, 121, 0, 1, 52, 1, 64, 1, 7, 114, 101, 113, 117, 101, 115, 116, 26, 0,
    32, 4, 24, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 45,
    104, 101, 97, 100, 101, 114, 115, 0, 1, 55, 1, 106, 1, 22, 0, 1, 64, 1, 7, 114, 101, 113, 117,
    101, 115, 116, 26, 0, 56, 4, 24, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117,
    101, 115, 116, 45, 99, 111, 110, 115, 117, 109, 101, 0, 1, 57, 1, 64, 6, 6, 109, 101, 116, 104,
    111, 100, 21, 4, 112, 97, 116, 104, 115, 5, 113, 117, 101, 114, 121, 115, 6, 115, 99, 104, 101,
    109, 101, 53, 9, 97, 117, 116, 104, 111, 114, 105, 116, 121, 115, 7, 104, 101, 97, 100, 101,
    114, 115, 32, 0, 19, 4, 20, 110, 101, 119, 45, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114,
    101, 113, 117, 101, 115, 116, 0, 1, 58, 1, 106, 1, 15, 0, 1, 64, 1, 7, 114, 101, 113, 117, 101,
    115, 116, 19, 0, 59, 4, 22, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113, 117,
    101, 115, 116, 45, 119, 114, 105, 116, 101, 0, 1, 60, 1, 64, 1, 8, 114, 101, 115, 112, 111,
    110, 115, 101, 11, 1, 0, 4, 22, 100, 114, 111, 112, 45, 114, 101, 115, 112, 111, 110, 115, 101,
    45, 111, 117, 116, 112, 97, 114, 97, 109, 0, 1, 61, 1, 106, 1, 17, 1, 34, 1, 106, 0, 0, 1, 64,
    1, 8, 114, 101, 115, 112, 111, 110, 115, 101, 62, 0, 63, 4, 21, 115, 101, 116, 45, 114, 101,
    115, 112, 111, 110, 115, 101, 45, 111, 117, 116, 112, 97, 114, 97, 109, 0, 1, 64, 1, 64, 1, 8,
    114, 101, 115, 112, 111, 110, 115, 101, 24, 1, 0, 4, 22, 100, 114, 111, 112, 45, 105, 110, 99,
    111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 1, 65, 1, 64, 1, 8,
    114, 101, 115, 112, 111, 110, 115, 101, 17, 1, 0, 4, 22, 100, 114, 111, 112, 45, 111, 117, 116,
    103, 111, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 1, 66, 1, 64, 1, 8,
    114, 101, 115, 112, 111, 110, 115, 101, 24, 0, 7, 4, 24, 105, 110, 99, 111, 109, 105, 110, 103,
    45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 115, 116, 97, 116, 117, 115, 0, 1, 67, 1, 64,
    1, 8, 114, 101, 115, 112, 111, 110, 115, 101, 24, 0, 32, 4, 25, 105, 110, 99, 111, 109, 105,
    110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 104, 101, 97, 100, 101, 114, 115, 0,
    1, 68, 1, 64, 1, 8, 114, 101, 115, 112, 111, 110, 115, 101, 24, 0, 56, 4, 25, 105, 110, 99,
    111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 99, 111, 110, 115,
    117, 109, 101, 0, 1, 69, 1, 64, 2, 11, 115, 116, 97, 116, 117, 115, 45, 99, 111, 100, 101, 7,
    7, 104, 101, 97, 100, 101, 114, 115, 32, 0, 17, 4, 21, 110, 101, 119, 45, 111, 117, 116, 103,
    111, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 1, 70, 1, 64, 1, 8, 114,
    101, 115, 112, 111, 110, 115, 101, 17, 0, 59, 4, 23, 111, 117, 116, 103, 111, 105, 110, 103,
    45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 119, 114, 105, 116, 101, 0, 1, 71, 1, 64, 1, 1,
    102, 28, 1, 0, 4, 29, 100, 114, 111, 112, 45, 102, 117, 116, 117, 114, 101, 45, 105, 110, 99,
    111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0, 1, 72, 1, 106, 1, 24,
    1, 34, 1, 107, 201, 0, 1, 64, 1, 1, 102, 28, 0, 202, 0, 4, 28, 102, 117, 116, 117, 114, 101,
    45, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 45, 103,
    101, 116, 0, 1, 75, 1, 64, 1, 1, 102, 28, 0, 5, 4, 34, 108, 105, 115, 116, 101, 110, 45, 116,
    111, 45, 102, 117, 116, 117, 114, 101, 45, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101,
    115, 112, 111, 110, 115, 101, 0, 1, 76, 3, 5, 116, 121, 112, 101, 115, 16, 112, 107, 103, 58,
    47, 116, 121, 112, 101, 115, 47, 116, 121, 112, 101, 115, 5, 7, 2, 3, 0, 4, 16, 111, 117, 116,
    103, 111, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 2, 3, 0, 4, 15, 114, 101, 113,
    117, 101, 115, 116, 45, 111, 112, 116, 105, 111, 110, 115, 2, 3, 0, 4, 24, 102, 117, 116, 117,
    114, 101, 45, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115,
    101, 1, 66, 9, 2, 3, 2, 1, 8, 4, 16, 111, 117, 116, 103, 111, 105, 110, 103, 45, 114, 101, 113,
    117, 101, 115, 116, 0, 3, 0, 0, 2, 3, 2, 1, 9, 4, 15, 114, 101, 113, 117, 101, 115, 116, 45,
    111, 112, 116, 105, 111, 110, 115, 0, 3, 0, 2, 2, 3, 2, 1, 10, 4, 24, 102, 117, 116, 117, 114,
    101, 45, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 115, 112, 111, 110, 115, 101, 0,
    3, 0, 4, 1, 107, 3, 1, 64, 2, 7, 114, 101, 113, 117, 101, 115, 116, 1, 7, 111, 112, 116, 105,
    111, 110, 115, 6, 0, 5, 4, 6, 104, 97, 110, 100, 108, 101, 0, 1, 7, 3, 21, 100, 101, 102, 97,
    117, 108, 116, 45, 111, 117, 116, 103, 111, 105, 110, 103, 45, 72, 84, 84, 80, 38, 112, 107,
    103, 58, 47, 111, 117, 116, 103, 111, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114, 47,
    111, 117, 116, 103, 111, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114, 5, 11, 2, 3, 0,
    4, 16, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101, 115, 116, 2, 3, 0,
    4, 17, 114, 101, 115, 112, 111, 110, 115, 101, 45, 111, 117, 116, 112, 97, 114, 97, 109, 1, 66,
    6, 2, 3, 2, 1, 12, 4, 16, 105, 110, 99, 111, 109, 105, 110, 103, 45, 114, 101, 113, 117, 101,
    115, 116, 0, 3, 0, 0, 2, 3, 2, 1, 13, 4, 17, 114, 101, 115, 112, 111, 110, 115, 101, 45, 111,
    117, 116, 112, 97, 114, 97, 109, 0, 3, 0, 2, 1, 64, 2, 7, 114, 101, 113, 117, 101, 115, 116, 1,
    12, 114, 101, 115, 112, 111, 110, 115, 101, 45, 111, 117, 116, 3, 1, 0, 4, 6, 104, 97, 110,
    100, 108, 101, 0, 1, 4, 4, 4, 72, 84, 84, 80, 38, 112, 107, 103, 58, 47, 105, 110, 99, 111,
    109, 105, 110, 103, 45, 104, 97, 110, 100, 108, 101, 114, 47, 105, 110, 99, 111, 109, 105, 110,
    103, 45, 104, 97, 110, 100, 108, 101, 114, 5, 14, 4, 5, 112, 114, 111, 120, 121, 16, 112, 107,
    103, 58, 47, 112, 114, 111, 120, 121, 47, 112, 114, 111, 120, 121, 4, 0, 0, 45, 9, 112, 114,
    111, 100, 117, 99, 101, 114, 115, 1, 12, 112, 114, 111, 99, 101, 115, 115, 101, 100, 45, 98,
    121, 1, 13, 119, 105, 116, 45, 99, 111, 109, 112, 111, 110, 101, 110, 116, 5, 48, 46, 54, 46,
    48, 11, 21, 1, 5, 112, 114, 111, 120, 121, 10, 112, 107, 103, 58, 47, 112, 114, 111, 120, 121,
    3, 6, 0,
];

#[inline(never)]
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub fn __link_section() {}
