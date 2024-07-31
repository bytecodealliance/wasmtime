#![cfg(not(miri))]

use super::REALLOC_AND_FREE;
use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store, StoreContextMut, Trap};

const UTF16_TAG: u32 = 1 << 31;

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
        let mut config = component_test_util::config();
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
        (memory $libc "memory")
        (realloc (func $libc "realloc"))
        string-encoding={encoding}
    ))
    (core instance $echo (instantiate $echo
        (with "libc" (instance $libc))
        (with "" (instance (export "echo" (func $echo))))
    ))
    (func (export "echo2") (param "a" string) (result string)
        (canon lift
            (core func $echo "echo")
            (memory $libc "memory")
            (realloc (func $libc "realloc"))
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
        func.post_return(&mut store)?;
    }
    Ok(())
}

#[test]
fn ptr_out_of_bounds() -> Result<()> {
    let engine = component_test_util::engine();
    for src in ENCODINGS {
        for dst in ENCODINGS {
            test_ptr_out_of_bounds(&engine, src, dst)?;
        }
    }
    Ok(())
}

fn test_ptr_out_of_bounds(engine: &Engine, src: &str, dst: &str) -> Result<()> {
    let test = |len: u32| -> Result<()> {
        let component = format!(
            r#"
(component
  (component $c
    (core module $m
      (func (export "") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
      (memory (export "memory") 1)
    )
    (core instance $m (instantiate $m))
    (func (export "a") (param "a" string)
      (canon lift (core func $m "") (realloc (func $m "realloc")) (memory $m "memory")
        string-encoding={dst})
    )
  )

  (component $c2
    (import "a" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
    )
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding={src} (memory $libc "memory")))
    (core module $m
      (import "" "" (func $f (param i32 i32)))

      (func $start (call $f (i32.const 0x8000_0000) (i32.const {len})))
      (start $start)
    )
    (core instance (instantiate $m (with "" (instance (export "" (func $f))))))
  )

  (instance $c (instantiate $c))
  (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
)
"#
        );
        let component = Component::new(engine, &component)?;
        let mut store = Store::new(engine, ());
        let trap = Linker::new(engine)
            .instantiate(&mut store, &component)
            .err()
            .unwrap()
            .downcast::<Trap>()?;
        assert_eq!(trap, Trap::UnreachableCodeReached);
        Ok(())
    };

    test(0)?;
    test(1)?;

    Ok(())
}

// Test that even if the ptr+len calculation overflows then a trap still
// happens.
#[test]
fn ptr_overflow() -> Result<()> {
    let engine = component_test_util::engine();
    for src in ENCODINGS {
        for dst in ENCODINGS {
            test_ptr_overflow(&engine, src, dst)?;
        }
    }
    Ok(())
}

fn test_ptr_overflow(engine: &Engine, src: &str, dst: &str) -> Result<()> {
    let component = format!(
        r#"
(component
  (component $c
    (core module $m
      (func (export "") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
      (memory (export "memory") 1)
    )
    (core instance $m (instantiate $m))
    (func (export "a") (param "a" string)
      (canon lift (core func $m "") (realloc (func $m "realloc")) (memory $m "memory")
        string-encoding={dst})
    )
  )

  (component $c2
    (import "a" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
    )
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding={src} (memory $libc "memory")))
    (core module $m
      (import "" "" (func $f (param i32 i32)))

      (func (export "f") (param i32) (call $f (i32.const 1000) (local.get 0)))
    )
    (core instance $m (instantiate $m (with "" (instance (export "" (func $f))))))
    (func (export "f") (param "a" u32) (canon lift (core func $m "f")))
  )

  (instance $c (instantiate $c))
  (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  (export "f" (func $c2 "f"))
)
"#
    );

    let component = Component::new(engine, &component)?;
    let mut store = Store::new(engine, ());

    let mut test_overflow = |size: u32| -> Result<()> {
        println!("src={src} dst={dst} size={size:#x}");
        let instance = Linker::new(engine).instantiate(&mut store, &component)?;
        let func = instance.get_typed_func::<(u32,), ()>(&mut store, "f")?;
        let trap = func
            .call(&mut store, (size,))
            .unwrap_err()
            .downcast::<Trap>()?;
        assert_eq!(trap, Trap::UnreachableCodeReached);
        Ok(())
    };

    let max = 1 << 31;

    match src {
        "utf8" => {
            // This exceeds MAX_STRING_BYTE_LENGTH
            test_overflow(max)?;

            if dst == "utf16" {
                // exceeds MAX_STRING_BYTE_LENGTH when multiplied
                test_overflow(max / 2)?;

                // Technically this fails on the first string, not the second.
                // Ideally this would test the overflow check on the second
                // string though.
                test_overflow(max / 2 - 100)?;
            } else {
                // This will point into unmapped memory
                test_overflow(max - 100)?;
            }
        }

        "utf16" => {
            test_overflow(max / 2)?;
            test_overflow(max / 2 - 100)?;
        }

        "latin1+utf16" => {
            test_overflow((max / 2) | UTF16_TAG)?;
            // tag a utf16 string with the max length and it should overflow.
            test_overflow((max / 2 - 100) | UTF16_TAG)?;
        }

        _ => unreachable!(),
    }

    Ok(())
}

// Test that that the pointer returned from `realloc` is bounds-checked.
#[test]
fn realloc_oob() -> Result<()> {
    let engine = component_test_util::engine();
    for src in ENCODINGS {
        for dst in ENCODINGS {
            test_realloc_oob(&engine, src, dst)?;
        }
    }
    Ok(())
}

fn test_realloc_oob(engine: &Engine, src: &str, dst: &str) -> Result<()> {
    let component = format!(
        r#"
(component
  (component $c
    (core module $m
      (func (export "") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 100_000)
      (memory (export "memory") 1)
    )
    (core instance $m (instantiate $m))
    (func (export "a") (param "a" string)
      (canon lift (core func $m "") (realloc (func $m "realloc")) (memory $m "memory")
        string-encoding={dst})
    )
  )

  (component $c2
    (import "a" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
    )
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding={src} (memory $libc "memory")))
    (core module $m
      (import "" "" (func $f (param i32 i32)))

      (func (export "f") (call $f (i32.const 1000) (i32.const 10)))
    )
    (core instance $m (instantiate $m (with "" (instance (export "" (func $f))))))
    (func (export "f") (canon lift (core func $m "f")))
  )

  (instance $c (instantiate $c))
  (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  (export "f" (func $c2 "f"))
)
"#
    );

    let component = Component::new(engine, &component)?;
    let mut store = Store::new(engine, ());

    let instance = Linker::new(engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "f")?;
    let trap = func.call(&mut store, ()).unwrap_err().downcast::<Trap>()?;
    assert_eq!(trap, Trap::UnreachableCodeReached);
    Ok(())
}

// Test that that the pointer returned from `realloc` is bounds-checked.
#[test]
fn raw_string_encodings() -> Result<()> {
    let engine = component_test_util::engine();
    test_invalid_string_encoding(&engine, "utf8", "utf8", &[0xff], 1)?;
    let array = b"valid string until \xffthen valid again";
    test_invalid_string_encoding(&engine, "utf8", "utf8", array, array.len() as u32)?;
    test_invalid_string_encoding(&engine, "utf8", "utf16", array, array.len() as u32)?;
    let array = b"symbol \xce\xa3 until \xffthen valid";
    test_invalid_string_encoding(&engine, "utf8", "utf8", array, array.len() as u32)?;
    test_invalid_string_encoding(&engine, "utf8", "utf16", array, array.len() as u32)?;
    test_invalid_string_encoding(&engine, "utf8", "latin1+utf16", array, array.len() as u32)?;
    test_invalid_string_encoding(&engine, "utf16", "utf8", &[0x01, 0xd8], 1)?;
    test_invalid_string_encoding(&engine, "utf16", "utf16", &[0x01, 0xd8], 1)?;
    test_invalid_string_encoding(
        &engine,
        "utf16",
        "latin1+utf16",
        &[0xff, 0xff, 0x01, 0xd8],
        2,
    )?;
    test_invalid_string_encoding(
        &engine,
        "latin1+utf16",
        "utf8",
        &[0x01, 0xd8],
        1 | UTF16_TAG,
    )?;
    test_invalid_string_encoding(
        &engine,
        "latin1+utf16",
        "utf16",
        &[0x01, 0xd8],
        1 | UTF16_TAG,
    )?;
    test_invalid_string_encoding(
        &engine,
        "latin1+utf16",
        "utf16",
        &[0xff, 0xff, 0x01, 0xd8],
        2 | UTF16_TAG,
    )?;
    test_invalid_string_encoding(
        &engine,
        "latin1+utf16",
        "latin1+utf16",
        &[0xab, 0x00, 0xff, 0xff, 0x01, 0xd8],
        3 | UTF16_TAG,
    )?;

    // This latin1+utf16 string should get compressed to latin1 across the
    // boundary.
    test_valid_string_encoding(
        &engine,
        "latin1+utf16",
        "latin1+utf16",
        &[0xab, 0x00, 0xff, 0x00],
        2 | UTF16_TAG,
    )?;
    Ok(())
}

fn test_invalid_string_encoding(
    engine: &Engine,
    src: &str,
    dst: &str,
    bytes: &[u8],
    len: u32,
) -> Result<()> {
    let trap = test_raw_when_encoded(engine, src, dst, bytes, len)?.unwrap();
    let src = src.replace("latin1+", "");
    assert!(
        format!("{trap:?}").contains(&format!("invalid {src} encoding")),
        "bad error: {trap:?}",
    );
    Ok(())
}

fn test_valid_string_encoding(
    engine: &Engine,
    src: &str,
    dst: &str,
    bytes: &[u8],
    len: u32,
) -> Result<()> {
    let err = test_raw_when_encoded(engine, src, dst, bytes, len)?;
    assert!(err.is_none());
    Ok(())
}

fn test_raw_when_encoded(
    engine: &Engine,
    src: &str,
    dst: &str,
    bytes: &[u8],
    len: u32,
) -> Result<Option<anyhow::Error>> {
    let component = format!(
        r#"
(component
  (component $c
    (core module $m
      (func (export "") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
      (memory (export "memory") 1)
    )
    (core instance $m (instantiate $m))
    (func (export "a") (param "a" string)
      (canon lift (core func $m "") (realloc (func $m "realloc")) (memory $m "memory")
        string-encoding={dst})
    )
  )

  (component $c2
    (import "a" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
    )
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding={src} (memory $libc "memory")))
    (core module $m
      (import "" "" (func $f (param i32 i32)))

      (func (export "f") (param i32 i32 i32) (call $f (local.get 0) (local.get 2)))
    )
    (core instance $m (instantiate $m (with "" (instance (export "" (func $f))))))
    (func (export "f") (param "a" (list u8)) (param "b" u32) (canon lift (core func $m "f")
        (memory $libc "memory")
        (realloc (func $libc "realloc"))))
  )

  (instance $c (instantiate $c))
  (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  (export "f" (func $c2 "f"))
)
"#
    );

    let component = Component::new(engine, &component)?;
    let mut store = Store::new(engine, ());

    let instance = Linker::new(engine).instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(&[u8], u32), ()>(&mut store, "f")?;
    match func.call(&mut store, (bytes, len)) {
        Ok(_) => Ok(None),
        Err(e) => Ok(Some(e)),
    }
}
