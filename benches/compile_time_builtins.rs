//! Measure accessing a host buffer via traditional host APIs and via compile-time builtins.

use criterion::{Bencher, Criterion, criterion_group, criterion_main};
use std::{mem, time::Instant};
use wasmtime::{component::Component, *};

const HOST_BUF_LEN: usize = 100_000;

/// A `*mut u8` pointer that is exposed directly to Wasm via unsafe intrinsics.
#[repr(align(8))]
union ExposedPointer {
    pointer: *mut u8,
    padding: u64,
}

const _EXPOSED_POINTER_LAYOUT_ASSERTIONS: () = {
    assert!(mem::size_of::<ExposedPointer>() == 8);
    assert!(mem::align_of::<ExposedPointer>() == 8);
};

impl ExposedPointer {
    /// Wrap the given pointer into an `ExposedPointer`.
    fn new(pointer: *mut u8) -> Self {
        // NB: Zero-initialize to avoid potential footguns with accessing
        // undefined bytes.
        let mut p = Self { padding: 0 };
        p.pointer = pointer;
        p
    }

    /// Get the wrapped pointer.
    fn get(&self) -> *mut u8 {
        unsafe { self.pointer }
    }
}

/// An owned `[u8]` slice that is exposed directly to Wasm via unsafe
/// intrinsics.
#[repr(C)]
struct ExposedBuf {
    buf_ptr: ExposedPointer,
    buf_len: u64,
}

const _EXPOSED_BUF_LAYOUT_ASSERTIONS: () = {
    assert!(mem::size_of::<ExposedBuf>() == 16);
    assert!(mem::align_of::<ExposedBuf>() == 8);
    assert!(mem::offset_of!(ExposedBuf, buf_ptr) == 0);
    assert!(mem::offset_of!(ExposedBuf, buf_len) == 8);
};

impl Drop for ExposedBuf {
    fn drop(&mut self) {
        let len = usize::try_from(self.buf_len).unwrap();
        let ptr = std::ptr::slice_from_raw_parts_mut(self.buf_ptr.get(), len);
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

impl ExposedBuf {
    /// Create a new `ExposedBuf`, allocating an inner buffer containing
    /// `bytes`.
    fn new(bytes: impl IntoIterator<Item = u8>) -> Self {
        let buf: Box<[u8]> = bytes.into_iter().collect();
        let ptr = Box::into_raw(buf);
        Self {
            buf_ptr: ExposedPointer::new(ptr.cast::<u8>()),
            buf_len: u64::try_from(ptr.len()).unwrap(),
        }
    }

    /// Get the inner buffer as a shared slice.
    fn get(&self) -> &[u8] {
        let ptr = self.buf_ptr.get().cast_const();
        let len = usize::try_from(self.buf_len).unwrap();
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }

    /// Get the inner buffer as an exclusive slice.
    fn get_mut(&mut self) -> &mut [u8] {
        let ptr = self.buf_ptr.get();
        let len = usize::try_from(self.buf_len).unwrap();
        unsafe { std::slice::from_raw_parts_mut(ptr, len) }
    }
}

fn make_engine() -> Result<Engine, Error> {
    let mut config = Config::new();
    config.compiler_inlining(true);
    config.concurrency_support(false);
    unsafe {
        config.cranelift_flag_set("wasmtime_inlining_intra_module", "yes");
    }
    let engine = Engine::new(&config)?;
    Ok(engine)
}

fn make_linker(engine: &Engine) -> Result<component::Linker<ExposedBuf>, Error> {
    let mut linker = wasmtime::component::Linker::<ExposedBuf>::new(engine);
    let mut host_buf = linker.instance("host-buf")?;

    host_buf.func_wrap("get", |ctx, (i,): (u64,)| -> Result<(u8,)> {
        let i = usize::try_from(i)?;
        match ctx.data().get().get(i).copied() {
            Some(x) => Ok((x,)),
            None => bail!("out-of-bounds host buffer access"),
        }
    })?;

    host_buf.func_wrap("set", |mut ctx, (i, x): (u64, u8)| -> Result<()> {
        let i = usize::try_from(i)?;
        match ctx.data_mut().get_mut().get_mut(i) {
            Some(b) => {
                *b = x;
                Ok(())
            }
            None => bail!("out-of-bounds host buffer access"),
        }
    })?;

    host_buf.func_wrap("len", |ctx, ()| -> Result<(u64,)> {
        let len = ctx.data().get().len();
        let len = u64::try_from(len)?;
        Ok((len,))
    })?;

    Ok(linker)
}

static COMPILE_TIME_BUILTINS: &str = r#"
    (component
        (import "unsafe-intrinsics"
            (instance $intrinsics
                (export "store-data-address" (func (result u64)))
                (export "u64-native-load" (func (param "pointer" u64) (result u64)))
                (export "u8-native-load" (func (param "pointer" u64) (result u8)))
                (export "u8-native-store" (func (param "pointer" u64) (param "value" u8)))
            )
        )

        ;; The core Wasm module that implements the safe API.
        (core module $host-buf-impl
            (import "" "store-data-address" (func $store-data-address (result i64)))
            (import "" "u64-native-load" (func $u64-native-load (param i64) (result i64)))
            (import "" "u8-native-load" (func $u8-native-load (param i64) (result i32)))
            (import "" "u8-native-store" (func $u8-native-store (param i64 i32)))

            ;; Load the `ExposedBuf::buf_ptr` field
            (func $get-buf-ptr (result i64)
                (call $u64-native-load (i64.add (call $store-data-address) (i64.const 0)))
            )

            ;; Load the `ExposedBuf::buf_len` field
            (func $get-buf-len (result i64)
                (call $u64-native-load (i64.add (call $store-data-address) (i64.const 8)))
            )

            ;; Check that `$i` is within `ExposedBuf` buffer's bounds, raising a trap
            ;; otherwise.
            (func $bounds-check (param $i i64)
                (if (i64.lt_u (local.get $i) (call $get-buf-len))
                    (then (return))
                    (else (unreachable))
                )
            )

            ;; A safe function to get the `i`th byte from `ExposedBuf`'s buffer,
            ;; raising a trap on out-of-bounds accesses.
            (func (export "get") (param $i i64) (result i32)
                (call $bounds-check (local.get $i))
                (call $u8-native-load (i64.add (call $get-buf-ptr) (local.get $i)))
            )

            ;; A safe function to set the `i`th byte in `ExposedBuf`'s buffer,
            ;; raising a trap on out-of-bounds accesses.
            (func (export "set") (param $i i64) (param $value i32)
                (call $bounds-check (local.get $i))
                (call $u8-native-store (i64.add (call $get-buf-ptr) (local.get $i))
                                       (local.get $value))
            )

            ;; A safe function to get the length of the `ExposedBuf` buffer.
            (func (export "len") (result i64)
                (call $get-buf-len)
            )
        )

        ;; Lower the imported intrinsics from component functions to core functions.
        (core func $store-data-address' (canon lower (func $intrinsics "store-data-address")))
        (core func $u64-native-load' (canon lower (func $intrinsics "u64-native-load")))
        (core func $u8-native-load' (canon lower (func $intrinsics "u8-native-load")))
        (core func $u8-native-store' (canon lower (func $intrinsics "u8-native-store")))

        ;; Instantiate our safe API implementation, passing in the lowered unsafe
        ;; intrinsics as its imports.
        (core instance $instance
            (instantiate $host-buf-impl
                (with "" (instance
                    (export "store-data-address" (func $store-data-address'))
                    (export "u64-native-load" (func $u64-native-load'))
                    (export "u8-native-load" (func $u8-native-load'))
                    (export "u8-native-store" (func $u8-native-store'))
                ))
            )
        )

        ;; Lift the safe API's exports from core functions to component functions
        ;; and export them.
        (func (export "get") (param "i" u64) (result u8)
            (canon lift (core func $instance "get"))
        )
        (func (export "set") (param "i" u64) (param "value" u8)
            (canon lift (core func $instance "set"))
        )
        (func (export "len") (result u64)
            (canon lift (core func $instance "len"))
        )
    )
"#;

mod inc_list {
    use super::*;

    pub fn bench(c: &mut Criterion) {
        let mut g = c.benchmark_group("increment-each-byte-in-buf");
        g.bench_function("func-wrap-host-buf-api", |b| {
            func_wrap_host_buf_api(b).unwrap();
        });
        g.bench_function("compile-time-builtins-host-buf-api", |b| {
            compile_time_builtins_host_buf_api(b).unwrap();
        });
        g.bench_function("take-and-return-list-u8", |b| {
            take_and_return_list_u8(b).unwrap();
        });
    }

    static HOST_BUF_API_GUEST_WASM: &str = r#"
        (component
            ;; Import the safe API.
            (import "host-buf"
                (instance $host-buf
                    (export "get" (func (param "i" u64) (result u8)))
                    (export "set" (func (param "i" u64) (param "value" u8)))
                    (export "len" (func (result u64)))
                )
            )

            ;; Define this component's core module implementation.
            (core module $main-impl
                (import "" "get" (func $get (param i64) (result i32)))
                (import "" "set" (func $set (param i64 i32)))
                (import "" "len" (func $len (result i64)))

                (func (export "main")
                    (local $i i64)
                    (local $n i64)

                    (local.set $i (i64.const 0))
                    (local.set $n (call $len))

                    (loop $loop
                        ;; When we have iterated over every byte in the
                        ;; buffer, exit.
                        (if (i64.ge_u (local.get $i) (local.get $n))
                            (then (return)))

                        ;; Increment the `i`th byte in the buffer.
                        (call $set (local.get $i)
                                   (i32.add (call $get (local.get $i))
                                            (i32.const 1)))

                        ;; Increment `i` and continue to the next iteration
                        ;; of the loop.
                        (local.set $i (i64.add (local.get $i) (i64.const 1)))
                        (br $loop)
                    )
                )
            )

            ;; Lower the imported safe APIs from component functions to core functions.
            (core func $get' (canon lower (func $host-buf "get")))
            (core func $set' (canon lower (func $host-buf "set")))
            (core func $len' (canon lower (func $host-buf "len")))

            ;; Instantiate our module, providing the lowered safe APIs as imports.
            (core instance $instance
                (instantiate $main-impl
                    (with "" (instance
                        (export "get" (func $get'))
                        (export "set" (func $set'))
                        (export "len" (func $len'))
                    ))
                )
            )

            ;; Lift the implementation's `main` from a core function to a component function
            ;; and export it!
            (func (export "main")
                (canon lift (core func $instance "main"))
            )
        )
    "#;

    fn func_wrap_host_buf_api(b: &mut Bencher<'_>) -> Result<()> {
        let engine = make_engine()?;
        let linker = make_linker(&engine)?;
        let component = Component::new(&engine, HOST_BUF_API_GUEST_WASM.as_bytes())?;

        let iter_func = |iters| -> Result<_> {
            let mut store = Store::new(
                &engine,
                ExposedBuf::new(std::iter::repeat_n(0u8, HOST_BUF_LEN)),
            );
            let instance = linker.instantiate(&mut store, &component)?;
            let main = instance.get_typed_func::<(), ()>(&mut store, "main")?;

            let start = Instant::now();
            for _ in 0..iters {
                main.call(&mut store, ())?;
            }
            let elapsed = start.elapsed();

            let expected = iters as u8;
            store.data().get().iter().enumerate().for_each(|(i, byte)| {
                assert_eq!(
                    *byte, expected,
                    "buf[{i}] = {byte}, but should be {expected}"
                )
            });

            Ok(elapsed)
        };
        b.iter_custom(|iters| iter_func(iters).unwrap());

        Ok(())
    }

    fn compile_time_builtins_host_buf_api(b: &mut Bencher<'_>) -> Result<()> {
        let engine = make_engine()?;
        let linker = make_linker(&engine)?;

        let mut builder = CodeBuilder::new(&engine);
        builder.wasm_binary_or_text(HOST_BUF_API_GUEST_WASM.as_bytes(), None)?;

        // Allow the code we are building to use Wasmtime's unsafe intrinsics.
        unsafe {
            builder.expose_unsafe_intrinsics("unsafe-intrinsics");
        }

        // Define the compile-time builtins that encapsulate the intrinsics'
        // unsafety and builds a safe API on top of them.
        unsafe {
            builder.compile_time_builtins_binary_or_text(
                "host-buf",
                COMPILE_TIME_BUILTINS.as_bytes(),
                None,
            )?;
        }

        let component = builder.compile_component()?;

        let iter_func = |iters| -> Result<_> {
            let mut store = Store::new(
                &engine,
                ExposedBuf::new(std::iter::repeat_n(0u8, HOST_BUF_LEN)),
            );
            let instance = linker.instantiate(&mut store, &component)?;
            let main = instance.get_typed_func::<(), ()>(&mut store, "main")?;

            let start = Instant::now();
            for _ in 0..iters {
                main.call(&mut store, ())?;
            }
            let elapsed = start.elapsed();

            let expected = iters as u8;
            store.data().get().iter().enumerate().for_each(|(i, byte)| {
                assert_eq!(
                    *byte, expected,
                    "buf[{i}] = {byte}, but should be {expected}"
                )
            });

            Ok(elapsed)
        };
        b.iter_custom(|iters| iter_func(iters).unwrap());

        Ok(())
    }

    static TAKE_AND_RETURN_INT_LIST_GUEST_WASM: &str = r#"
        (component
            (core module $m
                (memory (export "memory") 2)
                (func (export "realloc") (param i32 i32 i32 i32) (result i32)
                    ;; Reserve the first two `u32`s of memory for the return-values buffer.
                    (i32.const 8)
                )
                (func (export "inc-list") (param $ptr i32) (param $len i32) (result i32)
                    (local $i i32)
                    (local.set $i (i32.const 0))

                    (block $outer
                        (loop $loop
                            ;; When we have iterated over every byte in the
                            ;; buffer, exit.
                            (if (i32.ge_u (local.get $i) (local.get $len))
                                (then (br $outer)))

                            ;; Increment the `i`th byte in the buffer.
                            (i32.store8 (i32.add (local.get $ptr) (local.get $i))
                                        (i32.add (i32.load8_u (i32.add (local.get $ptr)
                                                                       (local.get $i))
                                                 (i32.const 1))))

                            ;; Increment `i` and continue to the next iteration
                            ;; of the loop.
                            (local.set $i (i32.add (local.get $i) (i32.const 1)))
                            (br $loop)
                        )
                    )

                    ;; Write the pointer and length into our reserved return-values buffer.
                    (i32.store offset=0 (i32.const 0) (local.get $ptr))
                    (i32.store offset=4 (i32.const 0) (local.get $len))
                    (i32.const 0)
                )
            )
            (core instance $i (instantiate $m))
            (func (export "inc-list") (param "xs" (list u8)) (result (list u8))
              (canon lift (core func $i "inc-list")
                          (memory $i "memory")
                          (realloc (func $i "realloc")))
            )
        )
    "#;

    fn take_and_return_list_u8(b: &mut Bencher<'_>) -> Result<()> {
        let engine = make_engine()?;
        let linker = make_linker(&engine)?;
        let component = Component::new(&engine, TAKE_AND_RETURN_INT_LIST_GUEST_WASM.as_bytes())?;

        let iter_func = |iters| -> Result<_> {
            let mut store = Store::new(&engine, ExposedBuf::new([]));
            let instance = linker.instantiate(&mut store, &component)?;
            let inc_list =
                instance.get_typed_func::<(&[u8],), (Vec<u8>,)>(&mut store, "inc-list")?;

            let mut buf = std::iter::repeat_n(0u8, HOST_BUF_LEN).collect::<Vec<u8>>();

            let start = Instant::now();
            for _ in 0..iters {
                buf = inc_list.call(&mut store, (&buf,))?.0;
            }
            let elapsed = start.elapsed();

            let expected = iters as u8;
            store.data().get().iter().enumerate().for_each(|(i, byte)| {
                assert_eq!(
                    *byte, expected,
                    "buf[{i}] = {byte}, but should be {expected}"
                )
            });

            Ok(elapsed)
        };
        b.iter_custom(|iters| iter_func(iters).unwrap());

        Ok(())
    }
}

mod inc_random {
    use super::*;
    use rand::{Rng, SeedableRng as _};

    pub fn bench(c: &mut Criterion) {
        let mut g = c.benchmark_group("increment-random-byte-in-buf");
        g.bench_function("func-wrap-host-buf-api", |b| {
            func_wrap_host_buf_api(b).unwrap();
        });
        g.bench_function("compile-time-builtins-host-buf-api", |b| {
            compile_time_builtins_host_buf_api(b).unwrap();
        });
        g.bench_function("take-and-return-list-u8", |b| {
            take_and_return_list_u8(b).unwrap();
        });
    }

    static HOST_BUF_API_GUEST_WASM: &str = r#"
        (component
            ;; Import the safe API.
            (import "host-buf"
                (instance $host-buf
                    (export "get" (func (param "i" u64) (result u8)))
                    (export "set" (func (param "i" u64) (param "value" u8)))
                )
            )

            ;; Define this component's core module implementation.
            (core module $inc-random-impl
                (import "" "get" (func $get (param i64) (result i32)))
                (import "" "set" (func $set (param i64 i32)))

                (func (export "inc-random") (param $i i64)
                    (call $set (local.get $i)
                               (i32.add (call $get (local.get $i))
                                        (i32.const 1)))
                )
            )

            ;; Lower the imported safe APIs from component functions to core functions.
            (core func $get' (canon lower (func $host-buf "get")))
            (core func $set' (canon lower (func $host-buf "set")))

            ;; Instantiate our module, providing the lowered safe APIs as imports.
            (core instance $instance
                (instantiate $inc-random-impl
                    (with "" (instance
                        (export "get" (func $get'))
                        (export "set" (func $set'))
                    ))
                )
            )

            ;; Lift the implementation's `inc-random` from a core function to a component function
            ;; and export it!
            (func (export "inc-random") (param "i" u64)
                (canon lift (core func $instance "inc-random"))
            )
        )
    "#;

    fn get_random_index() -> Result<u64, Error> {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
        let i = rng.random_range(0..HOST_BUF_LEN);
        let i = u64::try_from(i)?;
        Ok(i)
    }

    fn func_wrap_host_buf_api(b: &mut Bencher<'_>) -> Result<()> {
        let engine = make_engine()?;
        let linker = make_linker(&engine)?;
        let component = Component::new(&engine, HOST_BUF_API_GUEST_WASM.as_bytes())?;

        let iter_func = |iters| -> Result<_> {
            let mut store = Store::new(
                &engine,
                ExposedBuf::new(std::iter::repeat_n(0u8, HOST_BUF_LEN)),
            );
            let instance = linker.instantiate(&mut store, &component)?;
            let inc_random = instance.get_typed_func::<(u64,), ()>(&mut store, "inc-random")?;
            let i = get_random_index()?;

            let start = Instant::now();
            for _ in 0..iters {
                inc_random.call(&mut store, (i,))?;
            }
            let elapsed = start.elapsed();

            let expected = iters as u8;
            let actual = store.data().get()[usize::try_from(i)?];
            assert_eq!(
                actual, expected,
                "buf[{i}] = {actual}, but should be {expected}"
            );

            Ok(elapsed)
        };
        b.iter_custom(|iters| iter_func(iters).unwrap());

        Ok(())
    }

    fn compile_time_builtins_host_buf_api(b: &mut Bencher<'_>) -> Result<()> {
        let engine = make_engine()?;
        let linker = make_linker(&engine)?;

        let mut builder = CodeBuilder::new(&engine);
        builder.wasm_binary_or_text(HOST_BUF_API_GUEST_WASM.as_bytes(), None)?;

        // Allow the code we are building to use Wasmtime's unsafe intrinsics.
        unsafe {
            builder.expose_unsafe_intrinsics("unsafe-intrinsics");
        }

        // Define the compile-time builtins that encapsulate the intrinsics'
        // unsafety and builds a safe API on top of them.
        unsafe {
            builder.compile_time_builtins_binary_or_text(
                "host-buf",
                COMPILE_TIME_BUILTINS.as_bytes(),
                None,
            )?;
        }

        let component = builder.compile_component()?;

        let iter_func = |iters| -> Result<_> {
            let mut store = Store::new(
                &engine,
                ExposedBuf::new(std::iter::repeat_n(0u8, HOST_BUF_LEN)),
            );
            let instance = linker.instantiate(&mut store, &component)?;
            let inc_random = instance.get_typed_func::<(u64,), ()>(&mut store, "inc-random")?;
            let i = get_random_index()?;

            let start = Instant::now();
            for _ in 0..iters {
                inc_random.call(&mut store, (i,))?;
            }
            let elapsed = start.elapsed();

            let expected = iters as u8;
            let actual = store.data().get()[usize::try_from(i)?];
            assert_eq!(
                actual, expected,
                "buf[{i}] = {actual}, but should be {expected}"
            );

            Ok(elapsed)
        };
        b.iter_custom(|iters| iter_func(iters).unwrap());

        Ok(())
    }

    static TAKE_AND_RETURN_INT_LIST_GUEST_WASM: &str = r#"
        (component
            (core module $m
                (memory (export "memory") 2)
                (func (export "realloc") (param i32 i32 i32 i32) (result i32)
                    ;; Reserve the first two `u32`s of memory for the return-values buffer.
                    (i32.const 8)
                )
                (func (export "inc-random") (param $i i32) (param $ptr i32) (param $len i32) (result i32)
                    ;; Increment the `i`th byte in the buffer.
                    (i32.store8 (i32.add (local.get $ptr) (local.get $i))
                                (i32.add (i32.load8_u (i32.add (local.get $ptr)
                                                               (local.get $i)))
                                         (i32.const 1)))

                    ;; Write the pointer and length into our reserved return-values buffer.
                    (i32.store offset=0 (i32.const 0) (local.get $ptr))
                    (i32.store offset=4 (i32.const 0) (local.get $len))
                    (i32.const 0)
                )
            )
            (core instance $i (instantiate $m))
            (func (export "inc-random") (param "i" u32) (param "xs" (list u8)) (result (list u8))
              (canon lift (core func $i "inc-random")
                          (memory $i "memory")
                          (realloc (func $i "realloc")))
            )
        )
    "#;

    fn take_and_return_list_u8(b: &mut Bencher<'_>) -> Result<()> {
        let engine = make_engine()?;
        let linker = make_linker(&engine)?;
        let component = Component::new(&engine, TAKE_AND_RETURN_INT_LIST_GUEST_WASM.as_bytes())?;

        let iter_func = |iters| -> Result<_> {
            let mut store = Store::new(&engine, ExposedBuf::new([]));
            let instance = linker.instantiate(&mut store, &component)?;
            let inc_random =
                instance.get_typed_func::<(u32, &[u8]), (Vec<u8>,)>(&mut store, "inc-random")?;
            let i = u32::try_from(get_random_index()?)?;
            let mut buf = std::iter::repeat_n(0u8, HOST_BUF_LEN).collect::<Vec<u8>>();

            let start = Instant::now();
            for _ in 0..iters {
                buf = inc_random.call(&mut store, (i, &buf))?.0;
            }
            let elapsed = start.elapsed();

            let expected = iters as u8;
            let actual = buf[usize::try_from(i)?];
            assert_eq!(
                actual, expected,
                "buf[{i}] = {actual}, but should be {expected}"
            );

            Ok(elapsed)
        };
        b.iter_custom(|iters| iter_func(iters).unwrap());

        Ok(())
    }
}

fn bench(c: &mut Criterion) {
    let _ = env_logger::try_init();

    inc_list::bench(c);
    inc_random::bench(c);
}

criterion_group!(benches, bench);
criterion_main!(benches);
