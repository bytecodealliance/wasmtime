//! Measure accessing a host buffer via traditional host APIs and via compile-time builtins.

use criterion::{Bencher, Criterion, criterion_group, criterion_main};
use std::{
    mem,
    ops::{Deref, DerefMut},
};
use wasmtime::{
    component::{Component, Resource, ResourceAny, ResourceType},
    *,
};
use wasmtime_bench_util::Timer;

// The nearest power of two to 100,000 (2^17 = 131_072). Using a power of two
// lets the PRNG's high bits map directly onto the buffer without division.
const HOST_BUF_LEN: usize = 1 << 17;

// We'll generate random indices of the buffer by taking the high bits from a
// multiplicative congruence generator modulo 2^64. So we'll get the sequence
// x_0, x_1, ...  where x_n+1 = a * x_n mod 2^64 and the nth buffer access will
// be at index x_n > 17.
//
// Recommended as a "good" multiplier by Steele and Vigna mod 2^64 (see
// https://arxiv.org/pdf/2001.05304, page 18, table 7).
const MCG_MULTIPLIER: u64 = 0xf1357aea2e62a9c5;
const MCG_SEED: u64 = 1;
// The shift required to take the highest bits of a u64 get an index to the
// buffer.
const MCG_OUTPUT_SHIFT: u32 = u64::BITS - HOST_BUF_LEN.trailing_zeros();

/// A `*mut T` pointer that is exposed directly to Wasm via unsafe intrinsics.
#[repr(align(8))]
union ExposedPointer<T> {
    pointer: *mut T,
    padding: u64,
}

const _EXPOSED_POINTER_LAYOUT_ASSERTIONS: () = {
    assert!(mem::size_of::<ExposedPointer<u8>>() == 8);
    assert!(mem::align_of::<ExposedPointer<u8>>() == 8);
};

impl<T> ExposedPointer<T> {
    /// Wrap the given pointer into an `ExposedPointer`.
    fn new(pointer: *mut T) -> Self {
        // NB: Zero-initialize to avoid potential footguns with accessing
        // undefined bytes.
        let mut p = Self { padding: 0 };
        p.pointer = pointer;
        p
    }

    /// Get the wrapped pointer.
    fn get(&self) -> *mut T {
        unsafe { self.pointer }
    }
}

/// An owned `[u8]` slice that is exposed directly to Wasm via unsafe
/// intrinsics.
#[repr(C)]
struct ExposedBuf {
    buf_ptr: ExposedPointer<u8>,
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

/// An owned `Vec<T>` that is exposed directly to Wasm via unsafe intrinsics.
#[repr(C)]
struct ExposedVec<T> {
    ptr: ExposedPointer<T>,
    len: u64,
    cap: u64,
}

const _EXPOSED_VEC_LAYOUT_ASSERTIONS: () = {
    assert!(mem::size_of::<ExposedVec<ExposedBuf>>() == 24);
    assert!(mem::align_of::<ExposedVec<ExposedBuf>>() == 8);
    assert!(mem::offset_of!(ExposedVec<ExposedBuf>, ptr) == 0);
    assert!(mem::offset_of!(ExposedVec<ExposedBuf>, len) == 8);
    assert!(mem::offset_of!(ExposedVec<ExposedBuf>, cap) == 16);
};

impl<T> From<Vec<T>> for ExposedVec<T> {
    fn from(vec: Vec<T>) -> Self {
        let mut vec = mem::ManuallyDrop::new(vec);
        Self {
            ptr: ExposedPointer::new(vec.as_mut_ptr()),
            len: u64::try_from(vec.len()).unwrap(),
            cap: u64::try_from(vec.capacity()).unwrap(),
        }
    }
}

impl<T> Drop for ExposedVec<T> {
    fn drop(&mut self) {
        let len = usize::try_from(self.len).unwrap();
        let cap = usize::try_from(self.cap).unwrap();
        unsafe {
            let _ = Vec::from_raw_parts(self.ptr.get(), len, cap);
        }
    }
}

impl<T> Deref for ExposedVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        let ptr = self.ptr.get().cast_const();
        let len = usize::try_from(self.len).unwrap();
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

impl<T> DerefMut for ExposedVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        let ptr = self.ptr.get();
        let len = usize::try_from(self.len).unwrap();
        unsafe { std::slice::from_raw_parts_mut(ptr, len) }
    }
}

/// The store data: every host buffer, indexed by `host-buf` resource reps.
type HostBufs = ExposedVec<ExposedBuf>;

fn make_engine(concurrency_support: bool) -> Result<Engine, Error> {
    let mut config = Config::new();
    config.compiler_inlining(wasmtime::Inlining::Yes);
    config.concurrency_support(concurrency_support);
    let engine = Engine::new(&config)?;
    Ok(engine)
}

/// Get the host buffer that the `buf` resource refers to.
fn host_buf<'a>(bufs: &'a mut HostBufs, buf: &Resource<ExposedBuf>) -> Result<&'a mut ExposedBuf> {
    let index = usize::try_from(buf.rep())?;
    bufs.get_mut(index)
        .ok_or_else(|| format_err!("invalid host buffer"))
}

fn make_linker(engine: &Engine) -> Result<component::Linker<HostBufs>, Error> {
    let mut linker = component::Linker::<HostBufs>::new(engine);
    let mut host_buf_instance = linker.instance("host-buf")?;

    host_buf_instance.resource(
        "host-buf",
        ResourceType::host::<ExposedBuf>(),
        |_, _| Ok(()),
    )?;

    host_buf_instance.func_wrap(
        "new",
        |ctx, (index,): (u32,)| -> Result<(Resource<ExposedBuf>,)> {
            ensure!(
                usize::try_from(index)? < ctx.data().len(),
                "invalid host buffer index"
            );
            Ok((Resource::new_own(index),))
        },
    )?;

    host_buf_instance.func_wrap(
        "get",
        |mut ctx, (buf, i): (Resource<ExposedBuf>, u64)| -> Result<(u8,)> {
            let i = usize::try_from(i)?;
            match host_buf(ctx.data_mut(), &buf)?.get().get(i).copied() {
                Some(x) => Ok((x,)),
                None => bail!("out-of-bounds host buffer access"),
            }
        },
    )?;

    host_buf_instance.func_wrap(
        "set",
        |mut ctx, (buf, i, x): (Resource<ExposedBuf>, u64, u8)| -> Result<()> {
            let i = usize::try_from(i)?;
            match host_buf(ctx.data_mut(), &buf)?.get_mut().get_mut(i) {
                Some(b) => {
                    *b = x;
                    Ok(())
                }
                None => bail!("out-of-bounds host buffer access"),
            }
        },
    )?;

    host_buf_instance.func_wrap(
        "len",
        |mut ctx, (buf,): (Resource<ExposedBuf>,)| -> Result<(u64,)> {
            let len = host_buf(ctx.data_mut(), &buf)?.get().len();
            let len = u64::try_from(len)?;
            Ok((len,))
        },
    )?;

    Ok(linker)
}

/// Get the `name` function from a host-buf API guest's `api` instance export.
fn get_api_func<Params, Results>(
    store: &mut Store<HostBufs>,
    instance: &component::Instance,
    name: &str,
) -> Result<component::TypedFunc<Params, Results>>
where
    Params: component::ComponentNamedList + component::Lower,
    Results: component::ComponentNamedList + component::Lift,
{
    let api = instance
        .get_export_index(&mut *store, None, "api")
        .ok_or_else(|| format_err!("missing `api` export"))?;
    let func = instance
        .get_export_index(&mut *store, Some(&api), name)
        .ok_or_else(|| format_err!("missing `api` export `{name}`"))?;
    instance.get_typed_func(&mut *store, &func)
}

/// Instantiate a guest of the host-buf API in a new store containing a single,
/// zeroed host buffer, and get a handle to that buffer via the guest's
/// `new-host-buf` export.
fn instantiate_host_buf_api_guest(
    engine: &Engine,
    linker: &component::Linker<HostBufs>,
    component: &Component,
) -> Result<(Store<HostBufs>, component::Instance, ResourceAny)> {
    let bufs = vec![ExposedBuf::new(std::iter::repeat_n(0u8, HOST_BUF_LEN))];
    let mut store = Store::new(engine, HostBufs::from(bufs));
    let instance = linker.instantiate(&mut store, component)?;
    let new_host_buf =
        get_api_func::<(u32,), (ResourceAny,)>(&mut store, &instance, "new-host-buf")?;
    let (buf,) = new_host_buf.call(&mut store, (0,))?;
    Ok((store, instance, buf))
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

        ;; A host buffer. Its rep is an index into the store data's
        ;; `ExposedVec<ExposedBuf>`.
        (type $host-buf (resource (rep i32)))
        (core func $host-buf-new (canon resource.new $host-buf))

        ;; The core Wasm module that implements the safe API.
        ;;
        ;; This component defines the `host-buf` resource, so lifted `borrow`s
        ;; of it arrive as reps rather than handles.
        (core module $host-buf-impl
            (import "" "store-data-address" (func $store-data-address (result i64)))
            (import "" "u64-native-load" (func $u64-native-load (param i64) (result i64)))
            (import "" "u8-native-load" (func $u8-native-load (param i64) (result i32)))
            (import "" "u8-native-store" (func $u8-native-store (param i64 i32)))
            (import "" "host-buf-new" (func $host-buf-new (param i32) (result i32)))

            ;; Load the `ExposedVec::ptr` field
            (func $get-vec-ptr (result i64)
                (call $u64-native-load (i64.add (call $store-data-address) (i64.const 0)))
            )

            ;; Load the `ExposedVec::len` field
            (func $get-vec-len (result i64)
                (call $u64-native-load (i64.add (call $store-data-address) (i64.const 8)))
            )

            ;; Check that `$index` is within the `ExposedVec`'s bounds, raising
            ;; a trap otherwise.
            (func $vec-bounds-check (param $index i32)
                (if (i64.lt_u (i64.extend_i32_u (local.get $index)) (call $get-vec-len))
                    (then (return))
                    (else (unreachable))
                )
            )

            ;; Get the address of the `$buf`th `ExposedBuf` in the
            ;; `ExposedVec`, raising a trap on out-of-bounds accesses.
            (func $get-buf-addr (param $buf i32) (result i64)
                (call $vec-bounds-check (local.get $buf))
                (i64.add (call $get-vec-ptr)
                         ;; `size_of::<ExposedBuf>() == 16`
                         (i64.mul (i64.extend_i32_u (local.get $buf)) (i64.const 16)))
            )

            ;; Load the `$buf`th `ExposedBuf::buf_ptr` field
            (func $get-buf-ptr (param $buf i32) (result i64)
                (call $u64-native-load (i64.add (call $get-buf-addr (local.get $buf))
                                                (i64.const 0)))
            )

            ;; Load the `$buf`th `ExposedBuf::buf_len` field
            (func $get-buf-len (param $buf i32) (result i64)
                (call $u64-native-load (i64.add (call $get-buf-addr (local.get $buf))
                                                (i64.const 8)))
            )

            ;; Check that `$i` is within the `$buf`th `ExposedBuf` buffer's
            ;; bounds, raising a trap otherwise.
            (func $buf-bounds-check (param $buf i32) (param $i i64)
                (if (i64.lt_u (local.get $i) (call $get-buf-len (local.get $buf)))
                    (then (return))
                    (else (unreachable))
                )
            )

            ;; A safe function to create a `host-buf` resource for the
            ;; `$index`th `ExposedBuf`, raising a trap on out-of-bounds
            ;; indices.
            (func (export "new") (param $index i32) (result i32)
                (call $vec-bounds-check (local.get $index))
                (call $host-buf-new (local.get $index))
            )

            ;; A safe function to get the `i`th byte from an `ExposedBuf`'s
            ;; buffer, raising a trap on out-of-bounds accesses.
            (func (export "get") (param $buf i32) (param $i i64) (result i32)
                (call $buf-bounds-check (local.get $buf) (local.get $i))
                (call $u8-native-load (i64.add (call $get-buf-ptr (local.get $buf))
                                               (local.get $i)))
            )

            ;; A safe function to set the `i`th byte in an `ExposedBuf`'s
            ;; buffer, raising a trap on out-of-bounds accesses.
            (func (export "set") (param $buf i32) (param $i i64) (param $value i32)
                (call $buf-bounds-check (local.get $buf) (local.get $i))
                (call $u8-native-store (i64.add (call $get-buf-ptr (local.get $buf))
                                                (local.get $i))
                                       (local.get $value))
            )

            ;; A safe function to get the length of an `ExposedBuf`'s buffer.
            (func (export "len") (param $buf i32) (result i64)
                (call $get-buf-len (local.get $buf))
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
                    (export "host-buf-new" (func $host-buf-new))
                ))
            )
        )

        ;; Lift the safe API's core functions to component functions.
        (func $new (param "index" u32) (result (own $host-buf))
            (canon lift (core func $instance "new"))
        )
        (func $get (param "buf" (borrow $host-buf)) (param "i" u64) (result u8)
            (canon lift (core func $instance "get"))
        )
        (func $set (param "buf" (borrow $host-buf)) (param "i" u64) (param "value" u8)
            (canon lift (core func $instance "set"))
        )
        (func $len (param "buf" (borrow $host-buf)) (result u64)
            (canon lift (core func $instance "len"))
        )

        ;; Export the safe API as a nested instance, whose function types
        ;; refer to the instance's own `host-buf` resource type export.
        (component $api
            (import "host-buf-type" (type $host-buf (sub resource)))
            (import "new-func" (func $new (param "index" u32) (result (own $host-buf))))
            (import "get-func" (func $get (param "buf" (borrow $host-buf)) (param "i" u64) (result u8)))
            (import "set-func" (func $set (param "buf" (borrow $host-buf)) (param "i" u64) (param "value" u8)))
            (import "len-func" (func $len (param "buf" (borrow $host-buf)) (result u64)))
            (export $host-buf' "host-buf" (type $host-buf))
            (export "new" (func $new) (func (param "index" u32) (result (own $host-buf'))))
            (export "get" (func $get) (func (param "buf" (borrow $host-buf')) (param "i" u64) (result u8)))
            (export "set" (func $set) (func (param "buf" (borrow $host-buf')) (param "i" u64) (param "value" u8)))
            (export "len" (func $len) (func (param "buf" (borrow $host-buf')) (result u64)))
        )
        (instance $api (instantiate $api
            (with "host-buf-type" (type $host-buf))
            (with "new-func" (func $new))
            (with "get-func" (func $get))
            (with "set-func" (func $set))
            (with "len-func" (func $len))
        ))
        (export "host-buf" (instance $api))
    )
"#;

mod inc_list {
    use super::*;

    pub fn bench(c: &mut Criterion, concurrency_support: bool) {
        let mut g = c.benchmark_group(&format!(
            "increment-each-byte-in-buf/concurrency_support={concurrency_support}"
        ));
        g.bench_function("func-wrap-host-buf-api", |b| {
            func_wrap_host_buf_api(b, concurrency_support).unwrap();
        });
        g.bench_function("compile-time-builtins-host-buf-api", |b| {
            compile_time_builtins_host_buf_api(b, concurrency_support).unwrap();
        });
        g.bench_function("take-and-return-list-u8", |b| {
            take_and_return_list_u8(b, concurrency_support).unwrap();
        });
    }

    static HOST_BUF_API_GUEST_WASM: &str = r#"
        (component
            ;; Import the safe API.
            (import "host-buf"
                (instance $host-buf
                    (export "host-buf" (type $host-buf (sub resource)))
                    (export "new" (func (param "index" u32) (result (own $host-buf))))
                    (export "get" (func (param "buf" (borrow $host-buf)) (param "i" u64) (result u8)))
                    (export "set" (func (param "buf" (borrow $host-buf)) (param "i" u64) (param "value" u8)))
                    (export "len" (func (param "buf" (borrow $host-buf)) (result u64)))
                )
            )
            (alias export $host-buf "host-buf" (type $host-buf))

            ;; Define this component's core module implementation.
            (core module $main-impl
                (import "" "new" (func $new (param i32) (result i32)))
                (import "" "drop" (func $drop (param i32)))
                (import "" "get" (func $get (param i32 i64) (result i32)))
                (import "" "set" (func $set (param i32 i64 i32)))
                (import "" "len" (func $len (param i32) (result i64)))

                (func (export "new-host-buf") (param $index i32) (result i32)
                    (call $new (local.get $index))
                )

                (func (export "main") (param $buf i32)
                    (local $i i64)
                    (local $n i64)

                    (local.set $i (i64.const 0))
                    (local.set $n (call $len (local.get $buf)))

                    (loop $loop
                        ;; When we have iterated over every byte in the
                        ;; buffer, exit.
                        (if (i64.ge_u (local.get $i) (local.get $n))
                            (then
                                ;; Borrows must be dropped before returning.
                                (call $drop (local.get $buf))
                                (return)))

                        ;; Increment the `i`th byte in the buffer.
                        (call $set (local.get $buf)
                                   (local.get $i)
                                   (i32.add (call $get (local.get $buf) (local.get $i))
                                            (i32.const 1)))

                        ;; Increment `i` and continue to the next iteration
                        ;; of the loop.
                        (local.set $i (i64.add (local.get $i) (i64.const 1)))
                        (br $loop)
                    )
                )
            )

            ;; Lower the imported safe APIs from component functions to core functions.
            (core func $new' (canon lower (func $host-buf "new")))
            (core func $drop (canon resource.drop $host-buf))
            (core func $get' (canon lower (func $host-buf "get")))
            (core func $set' (canon lower (func $host-buf "set")))
            (core func $len' (canon lower (func $host-buf "len")))

            ;; Instantiate our module, providing the lowered safe APIs as imports.
            (core instance $instance
                (instantiate $main-impl
                    (with "" (instance
                        (export "new" (func $new'))
                        (export "drop" (func $drop))
                        (export "get" (func $get'))
                        (export "set" (func $set'))
                        (export "len" (func $len'))
                    ))
                )
            )

            ;; Lift the implementation's `new-host-buf` and `main` from core
            ;; functions to component functions.
            (func $new-host-buf (param "index" u32) (result (own $host-buf))
                (canon lift (core func $instance "new-host-buf"))
            )
            (func $main (param "buf" (borrow $host-buf))
                (canon lift (core func $instance "main"))
            )

            ;; Export them, along with the host buffer resource type, as a
            ;; nested instance. Composition re-exports a nested instance as-is,
            ;; but would re-export top-level functions without the type
            ;; ascriptions tying their `borrow`s to the re-exported resource
            ;; type, which is invalid.
            (component $api
                (import "host-buf-type" (type $host-buf (sub resource)))
                (import "new-host-buf-func" (func $new-host-buf (param "index" u32) (result (own $host-buf))))
                (import "main-func" (func $main (param "buf" (borrow $host-buf))))
                (export $host-buf' "host-buf" (type $host-buf))
                (export "new-host-buf" (func $new-host-buf) (func (param "index" u32) (result (own $host-buf'))))
                (export "main" (func $main) (func (param "buf" (borrow $host-buf'))))
            )
            (instance $api (instantiate $api
                (with "host-buf-type" (type $host-buf))
                (with "new-host-buf-func" (func $new-host-buf))
                (with "main-func" (func $main))
            ))
            (export "api" (instance $api))
        )
    "#;

    fn func_wrap_host_buf_api(b: &mut Bencher<'_>, concurrency_support: bool) -> Result<()> {
        let engine = make_engine(concurrency_support)?;
        let linker = make_linker(&engine)?;
        let component = Component::new(&engine, HOST_BUF_API_GUEST_WASM.as_bytes())?;

        let iter_func = |iters| -> Result<_> {
            let (mut store, instance, buf) =
                instantiate_host_buf_api_guest(&engine, &linker, &component)?;
            let main = get_api_func::<(ResourceAny,), ()>(&mut store, &instance, "main")?;

            let start = Timer::start();
            for _ in 0..iters {
                main.call(&mut store, (buf,))?;
            }
            let elapsed = start.stop();

            buf.resource_drop(&mut store)?;
            let expected = iters as u8;
            store.data()[0]
                .get()
                .iter()
                .enumerate()
                .for_each(|(i, byte)| {
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

    fn compile_time_builtins_host_buf_api(
        b: &mut Bencher<'_>,
        concurrency_support: bool,
    ) -> Result<()> {
        let engine = make_engine(concurrency_support)?;
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
            let (mut store, instance, buf) =
                instantiate_host_buf_api_guest(&engine, &linker, &component)?;
            let main = get_api_func::<(ResourceAny,), ()>(&mut store, &instance, "main")?;

            let start = Timer::start();
            for _ in 0..iters {
                main.call(&mut store, (buf,))?;
            }
            let elapsed = start.stop();

            buf.resource_drop(&mut store)?;
            let expected = iters as u8;
            store.data()[0]
                .get()
                .iter()
                .enumerate()
                .for_each(|(i, byte)| {
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
                (memory (export "memory") 3)
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
                                                                       (local.get $i)))
                                                 (i32.const 1)))

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
                          (memory (core memory $i "memory"))
                          (realloc (core func $i "realloc")))
            )
        )
    "#;

    fn take_and_return_list_u8(b: &mut Bencher<'_>, concurrency_support: bool) -> Result<()> {
        let engine = make_engine(concurrency_support)?;
        let linker = make_linker(&engine)?;
        let component = Component::new(&engine, TAKE_AND_RETURN_INT_LIST_GUEST_WASM.as_bytes())?;

        let iter_func = |iters| -> Result<_> {
            let mut store = Store::new(&engine, HostBufs::from(vec![]));
            let instance = linker.instantiate(&mut store, &component)?;
            let inc_list =
                instance.get_typed_func::<(&[u8],), (Vec<u8>,)>(&mut store, "inc-list")?;

            let mut buf = std::iter::repeat_n(0u8, HOST_BUF_LEN).collect::<Vec<u8>>();

            let start = Timer::start();
            for _ in 0..iters {
                buf = inc_list.call(&mut store, (&buf,))?.0;
            }
            let elapsed = start.stop();

            let expected = iters as u8;
            assert_eq!(buf.len(), HOST_BUF_LEN);
            buf.iter().enumerate().for_each(|(i, byte)| {
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
    use rand::{RngExt, SeedableRng as _};

    pub fn bench(c: &mut Criterion, concurrency_support: bool) {
        let mut g = c.benchmark_group(&format!(
            "increment-random-byte-in-buf/concurrency_support={concurrency_support}"
        ));
        g.bench_function("func-wrap-host-buf-api", |b| {
            func_wrap_host_buf_api(b, concurrency_support).unwrap();
        });
        g.bench_function("compile-time-builtins-host-buf-api", |b| {
            compile_time_builtins_host_buf_api(b, concurrency_support).unwrap();
        });
        g.bench_function("take-and-return-list-u8", |b| {
            take_and_return_list_u8(b, concurrency_support).unwrap();
        });
    }

    fn host_buf_api_guest_wasm() -> String {
        format!(
            r#"
        (component
            ;; Import the safe API.
            (import "host-buf"
                (instance $host-buf
                    (export "host-buf" (type $host-buf (sub resource)))
                    (export "new" (func (param "index" u32) (result (own $host-buf))))
                    (export "get" (func (param "buf" (borrow $host-buf)) (param "i" u64) (result u8)))
                    (export "set" (func (param "buf" (borrow $host-buf)) (param "i" u64) (param "value" u8)))
                )
            )
            (alias export $host-buf "host-buf" (type $host-buf))

            ;; Define this component's core module implementation.
            (core module $inc-random-impl
                (import "" "new" (func $new (param i32) (result i32)))
                (import "" "drop" (func $drop (param i32)))
                (import "" "get" (func $get (param i32 i64) (result i32)))
                (import "" "set" (func $set (param i32 i64 i32)))

                (func (export "new-host-buf") (param $index i32) (result i32)
                    (call $new (local.get $index))
                )

                (func (export "inc-random")
                    (param $buf i32)
                    (param $iters i64)
                    (local $state i64)
                    (local $i i64)

                    (local.set $state (i64.const {MCG_SEED}))
                    loop $l
                        ;; Take the highest bits of random state for the index.
                        (local.set $i
                            (i64.shr_u
                                (local.get $state)
                                (i64.const {MCG_OUTPUT_SHIFT})
                            )
                        )
                        ;; Increment the buffer.
                        (call $set (local.get $buf)
                                   (local.get $i)
                                   (i32.add (call $get (local.get $buf) (local.get $i))
                                            (i32.const 1)))
                        (local.set $iters (i64.sub (local.get $iters) (i64.const 1)))
                        local.get $iters
                        i64.eqz
                        if
                          ;; Borrows must be dropped before returning.
                          (call $drop (local.get $buf))
                          return
                        end
                        ;; Get next random number by multiplying by multiplier.
                        (local.set $state
                            (i64.mul
                                (local.get $state)
                                (i64.const {MCG_MULTIPLIER})
                            )
                        )
                        br $l
                    end
                )
            )

            ;; Lower the imported safe APIs from component functions to core functions.
            (core func $new' (canon lower (func $host-buf "new")))
            (core func $drop (canon resource.drop $host-buf))
            (core func $get' (canon lower (func $host-buf "get")))
            (core func $set' (canon lower (func $host-buf "set")))

            ;; Instantiate our module, providing the lowered safe APIs as imports.
            (core instance $instance
                (instantiate $inc-random-impl
                    (with "" (instance
                        (export "new" (func $new'))
                        (export "drop" (func $drop))
                        (export "get" (func $get'))
                        (export "set" (func $set'))
                    ))
                )
            )

            ;; Lift the implementation's `new-host-buf` and `inc-random` from
            ;; core functions to component functions.
            (func $new-host-buf (param "index" u32) (result (own $host-buf))
                (canon lift (core func $instance "new-host-buf"))
            )
            (func $inc-random (param "buf" (borrow $host-buf)) (param "iters" u64)
                (canon lift (core func $instance "inc-random"))
            )

            ;; Export them, along with the host buffer resource type, as a
            ;; nested instance; see the comment in `inc_list`'s guest.
            (component $api
                (import "host-buf-type" (type $host-buf (sub resource)))
                (import "new-host-buf-func" (func $new-host-buf (param "index" u32) (result (own $host-buf))))
                (import "inc-random-func" (func $inc-random (param "buf" (borrow $host-buf)) (param "iters" u64)))
                (export $host-buf' "host-buf" (type $host-buf))
                (export "new-host-buf" (func $new-host-buf) (func (param "index" u32) (result (own $host-buf'))))
                (export "inc-random" (func $inc-random) (func (param "buf" (borrow $host-buf')) (param "iters" u64)))
            )
            (instance $api (instantiate $api
                (with "host-buf-type" (type $host-buf))
                (with "new-host-buf-func" (func $new-host-buf))
                (with "inc-random-func" (func $inc-random))
            ))
            (export "api" (instance $api))
        )
    "#
        )
    }

    fn get_random_index() -> Result<u64, Error> {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
        let i = rng.random_range(0..HOST_BUF_LEN);
        let i = u64::try_from(i)?;
        Ok(i)
    }

    fn get_expected(iters: u64) -> Vec<u8> {
        let mut state = MCG_SEED;
        let mut buf = vec![0u8; HOST_BUF_LEN];
        for _ in 0..iters {
            let i = usize::try_from(state >> MCG_OUTPUT_SHIFT).unwrap();
            buf[i] = buf[i].wrapping_add(1);
            state = state.wrapping_mul(MCG_MULTIPLIER);
        }
        buf
    }

    fn func_wrap_host_buf_api(b: &mut Bencher<'_>, concurrency_support: bool) -> Result<()> {
        let engine = make_engine(concurrency_support)?;
        let linker = make_linker(&engine)?;
        let guest_wasm = host_buf_api_guest_wasm();
        let component = Component::new(&engine, guest_wasm.as_bytes())?;

        let iter_func = |iters| -> Result<_> {
            let (mut store, instance, buf) =
                instantiate_host_buf_api_guest(&engine, &linker, &component)?;
            let inc_random =
                get_api_func::<(ResourceAny, u64), ()>(&mut store, &instance, "inc-random")?;

            let start = Timer::start();
            inc_random.call(&mut store, (buf, iters))?;
            let elapsed = start.stop();

            buf.resource_drop(&mut store)?;
            let expected = get_expected(iters);
            let actual = store.data()[0].get();
            assert_eq!(actual, expected);

            Ok(elapsed)
        };
        b.iter_custom(|iters| iter_func(iters).unwrap());

        Ok(())
    }

    fn compile_time_builtins_host_buf_api(
        b: &mut Bencher<'_>,
        concurrency_support: bool,
    ) -> Result<()> {
        let engine = make_engine(concurrency_support)?;
        let linker = make_linker(&engine)?;

        let mut builder = CodeBuilder::new(&engine);
        let guest_wasm = host_buf_api_guest_wasm();
        builder.wasm_binary_or_text(guest_wasm.as_bytes(), None)?;

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
            let (mut store, instance, buf) =
                instantiate_host_buf_api_guest(&engine, &linker, &component)?;
            let inc_random =
                get_api_func::<(ResourceAny, u64), ()>(&mut store, &instance, "inc-random")?;

            let start = Timer::start();
            inc_random.call(&mut store, (buf, iters))?;
            let elapsed = start.stop();

            buf.resource_drop(&mut store)?;
            let expected = get_expected(iters);
            let actual = store.data()[0].get();
            assert_eq!(actual, expected);

            Ok(elapsed)
        };
        b.iter_custom(|iters| iter_func(iters).unwrap());

        Ok(())
    }

    static TAKE_AND_RETURN_INT_LIST_GUEST_WASM: &str = r#"
        (component
            (core module $m
                (memory (export "memory") 3)
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
                          (memory (core memory $i "memory"))
                          (realloc (core func $i "realloc")))
            )
        )
    "#;

    fn take_and_return_list_u8(b: &mut Bencher<'_>, concurrency_support: bool) -> Result<()> {
        let engine = make_engine(concurrency_support)?;
        let linker = make_linker(&engine)?;
        let component = Component::new(&engine, TAKE_AND_RETURN_INT_LIST_GUEST_WASM.as_bytes())?;

        let iter_func = |iters| -> Result<_> {
            let mut store = Store::new(&engine, HostBufs::from(vec![]));
            let instance = linker.instantiate(&mut store, &component)?;
            let inc_random =
                instance.get_typed_func::<(u32, &[u8]), (Vec<u8>,)>(&mut store, "inc-random")?;
            let i = u32::try_from(get_random_index()?)?;
            let mut buf = std::iter::repeat_n(0u8, HOST_BUF_LEN).collect::<Vec<u8>>();

            let start = Timer::start();
            for _ in 0..iters {
                buf = inc_random.call(&mut store, (i, &buf))?.0;
            }
            let elapsed = start.stop();

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

    for concurrency_support in [true, false] {
        inc_list::bench(c, concurrency_support);
        inc_random::bench(c, concurrency_support);
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
