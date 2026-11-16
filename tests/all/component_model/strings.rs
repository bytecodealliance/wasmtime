#![cfg(not(miri))]

use super::REALLOC_AND_FREE;
use wasmtime::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, PoolingAllocationConfig, Store, StoreContextMut};

// Special cases that this tries to test:
//
// * utf8 -> utf8
//    * various code point sizes
//
// * utf8 -> utf16 - the adapter here will make a pessimistic allocation that's
//   twice the size of the utf8 encoding for the utf16 destination
//    * utf16 byte size is twice the utf8 size
//    * utf16 byte size is less than twice the utf8 size
//
// * utf8 -> latin1+utf16 - attempts to convert to latin1 then falls back to a
//   pessimistic utf16 allocation that's downsized if necessary
//    * utf8 fits exactly in latin1
//    * utf8 fits latin1 but is bigger byte-wise
//    * utf8 is not latin1 and fits utf16 allocation precisely (NOT POSSIBLE)
//    * utf8 is not latin1 and utf16 is smaller than allocation
//
// * utf16 -> utf8 - this starts with an optimistic size and then reallocates to
//   a pessimistic size, interesting cases are:
//    * utf8 size is 0.5x the utf16 byte size (perfect fit in initial alloc)
//    * utf8 size is 1.5x the utf16 byte size (perfect fit in larger alloc)
//    * utf8 size is 0.5x-1.5x the utf16 size (larger alloc is downsized)
//
// * utf16 -> utf16
//    * various code point sizes
//
// * utf16 -> latin1+utf16 - attempts to convert to latin1 then falls back to a
//   pessimistic utf16 allocation that's downsized if necessary
//    * utf16 fits exactly in latin1
//    * utf16 fits latin1 but is bigger byte-wise (NOT POSSIBLE)
//    * utf16 is not latin1 and fits utf16 allocation precisely
//    * utf16 is not latin1 and utf16 is smaller than allocation (NOT POSSIBLE)
//
// * compact-utf16 -> utf8 dynamically determines between one of
//    * latin1 -> utf8
//      * latin1 size matches utf8 size
//      * latin1 is smaller than utf8 size
//    * utf16 -> utf8
//      * covered above
//
// * compact-utf16 -> utf16 dynamically determines between one of
//    * latin1 -> utf16 - latin1 size always matches utf16
//      * test various code points
//    * utf16 -> utf16
//      * covered above
//
// * compact-utf16 -> compact-utf16 dynamically determines between one of
//    * latin1 -> latin1
//      * not much interesting here
//    * utf16 -> compact-utf16-to-compact-probably-utf16
//      * utf16 actually fits within latin1
//      * otherwise not more interesting than utf16 -> utf16
//
const STRINGS: &[&str] = &[
    "",
    // 1 byte in utf8, 2 bytes in utf16
    "x",
    "hello this is a particularly long string yes it is it keeps going",
    // 35 bytes in utf8, 23 units in utf16, 23 bytes in latin1
    "à á â ã ä å æ ç è é ê ë",
    // 47 bytes in utf8, 31 units in utf16
    "Ξ Ο Π Ρ Σ Τ Υ Φ Χ Ψ Ω Ϊ Ϋ ά έ ή",
    // 24 bytes in utf8, 8 units in utf16
    "ＳＴＵＶＷＸＹＺ",
    // 16 bytes in utf8, 8 units in utf16
    "ËÌÍÎÏÐÑÒ",
    // 4 bytes in utf8, 1 unit in utf16
    "\u{10000}",
    // latin1-compatible prefix followed by utf8/16-requiring suffix
    //
    // 24 bytes in utf8, 13 units in utf16, first 8 usvs are latin1-compatible
    "à ascii ＶＷＸＹＺ",
];

static ENCODINGS: [&str; 3] = ["utf8", "utf16", "latin1+utf16"];

#[test]
fn roundtrip() -> Result<()> {
    for debug in [true, false] {
        let mut config = wasmtime_test_util::component::config();
        config.debug_adapter_modules(debug);
        let engine = Engine::new(&config)?;
        for src in ENCODINGS {
            for dst in ENCODINGS {
                test_roundtrip(&engine, src, dst)?;
            }
        }
    }
    Ok(())
}

fn test_roundtrip(engine: &Engine, src: &str, dst: &str) -> Result<()> {
    println!("src={src} dst={dst}");

    let mk_echo = |name: &str, encoding: &str| {
        format!(
            r#"
(component {name}
    (import "echo" (func $echo (param "a" string) (result string)))
    (core instance $libc (instantiate $libc))
    (core func $echo (canon lower (func $echo)
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))
        string-encoding={encoding}
    ))
    (core instance $echo (instantiate $echo
        (with "libc" (instance $libc))
        (with "" (instance (export "echo" (func $echo))))
    ))
    (func (export "echo2") (param "a" string) (result string)
        (canon lift
            (core func $echo "echo")
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
            string-encoding={encoding}
        )
    )
)
            "#
        )
    };

    let src = mk_echo("$src", src);
    let dst = mk_echo("$dst", dst);
    let component = format!(
        r#"
(component
    (import "host" (func $host (param "a" string) (result string)))

    (core module $libc
        (memory (export "memory") 1)
        {REALLOC_AND_FREE}
    )
    (core module $echo
        (import "" "echo" (func $echo (param i32 i32 i32)))
        (import "libc" "memory" (memory 0))
        (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))

        (func (export "echo") (param i32 i32) (result i32)
            (local $retptr i32)
            (local.set $retptr
                (call $realloc
                    (i32.const 0)
                    (i32.const 0)
                    (i32.const 4)
                    (i32.const 8)))
            (call $echo
                (local.get 0)
                (local.get 1)
                (local.get $retptr))
            local.get $retptr
        )
    )

    {src}
    {dst}

    (instance $dst (instantiate $dst (with "echo" (func $host))))
    (instance $src (instantiate $src (with "echo" (func $dst "echo2"))))
    (export "echo" (func $src "echo2"))
)
"#
    );
    let component = Component::new(engine, &component)?;
    let mut store = Store::new(engine, String::new());
    let mut linker = Linker::new(engine);
    linker.root().func_wrap(
        "host",
        |store: StoreContextMut<String>, (arg,): (String,)| {
            assert_eq!(*store.data(), arg);
            Ok((arg,))
        },
    )?;
    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(String,), (String,)>(&mut store, "echo")?;

    for string in STRINGS {
        println!("testing string {string:?}");
        *store.data_mut() = string.to_string();
        let (ret,) = func.call(&mut store, (string.to_string(),))?;
        assert_eq!(ret, *string);
    }
    Ok(())
}

#[test]
fn pass_string_on_component_boundary() -> Result<()> {
    // Configure an engine such that linear memories are allocated right next
    // to each other and are 1 wasm page large. This'll ensure that the string
    // at the beginning of the second memory is at the end of the first memory.
    let mut pooling_config = PoolingAllocationConfig::new();
    pooling_config.total_component_instances(3);
    pooling_config.total_memories(2);
    pooling_config.total_tables(0);
    pooling_config.total_stacks(0);
    pooling_config.max_memory_size(65536);
    let mut config = Config::new();
    config.memory_guard_size(0);
    config.memory_reservation(65536);
    config.allocation_strategy(pooling_config);
    let engine = Engine::new(&config)?;

    let component = r#"
(component
  ;; This component is instantiated first so its allocation function returns a
  ;; pointer at the end of memory which will be right up against the next
  ;; linear memory.
  (component $c
    (core module $m
      (func (export "") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 65520)
      (memory (export "memory") 1)
    )
    (core instance $m (instantiate $m))
    (func (export "a") (param "a" string)
      (canon lift (core func $m "") (realloc (core func $m "realloc")) (memory (core memory $m "memory")))
    )
  )

  ;; This component is instantiated second meaning its memory is after the
  ;; one above, so the string is placed first thing in linear memory.
  (component $c2
    (import "a" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
      (data (memory 0) (i32.const 0) "0123456789abcdef")
    )
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) (memory (core memory $libc "memory"))))
    (core module $m
      (import "" "" (func $f (param i32 i32)))
      (func (export "f")
        (call $f
          (i32.const 0)   ;; ptr
          (i32.const 16)) ;; len
      )
    )
    (core instance $m (instantiate $m (with "" (instance (export "" (func $f))))))
    (func (export "f") (canon lift (core func $m "f")))
  )

  (instance $c (instantiate $c))
  (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  (export "f" (func $c2 "f"))
)
"#;

    let component = Component::new(&engine, &component)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "f")?;
    func.call(&mut store, ())?;
    Ok(())
}
