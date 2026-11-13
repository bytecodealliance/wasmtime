//! A host-created `StreamReader<T>` from store A, lowered into a guest of store
//! B, resolves to whatever stream B has at the same host-table index and
//! generation. On this branch that reader is `TransmitCheck::Elided`, so no
//! check compares B's producer's items with the guest's payload type.
//!
//! Usage: cross-store-reader-poc <mismatch|match>
//!   mismatch: A has `StreamReader<u32>`, B's stream produces `String`s.
//!   match:    both produce `u32`s (control: shows the pre-existing aliasing).

use wasmtime::component::{Component, Instance, Linker, StreamReader};
use wasmtime::{Config, Engine, Result, Store};

const GUEST: &str = r#"
(component
    (type $u (stream u32))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $task.return (canon task.return (result u32)))
    (core func $read.u (canon stream.read $u async (memory (core memory $libc "mem"))))
    (core func $ws.new (canon waitable-set.new))
    (core func $join (canon waitable.join))
    (core module $m
        (import "" "mem" (memory 1))
        (import "" "task.return" (func $task.return (param i32)))
        (import "" "read.u" (func $read.u (param i32 i32 i32) (result i32)))
        (import "" "ws.new" (func $ws.new (result i32)))
        (import "" "join" (func $join (param i32 i32)))
        (global $s (mut i32) (i32.const 0))
        (global $ws (mut i32) (i32.const 0))
        (global $started (mut i32) (i32.const 0))
        ;; Reads one `u32` from the stream and returns it.
        (func $step (result i32)
            (if (i32.eqz (global.get $started))
                (then
                    (global.set $started (i32.const 1))
                    (if (i32.eq (call $read.u (global.get $s) (i32.const 16) (i32.const 1))
                                (i32.const -1))
                        (then
                            (call $join (global.get $s) (global.get $ws))
                            (return (i32.or (i32.const 2)
                                            (i32.shl (global.get $ws) (i32.const 4))))))))
            (call $task.return (i32.load (i32.const 16)))
            (i32.const 0))
        (func (export "consume-u") (param i32) (result i32)
            (global.set $s (local.get 0))
            (global.set $ws (call $ws.new))
            (call $step))
        (func (export "consume-cb") (param i32 i32 i32) (result i32) (call $step))
    )
    (core instance $i (instantiate $m (with "" (instance
        (export "mem" (memory $libc "mem"))
        (export "task.return" (func $task.return))
        (export "read.u" (func $read.u))
        (export "ws.new" (func $ws.new))
        (export "join" (func $join))
    ))))
    (func (export "consume-u") async (param "s" $u) (result u32)
        (canon lift (core func $i "consume-u") async (memory (core memory $libc "mem"))
            (callback (core func $i "consume-cb"))))
)
"#;

async fn instance(engine: &Engine, component: &Component) -> Result<(Store<()>, Instance)> {
    let mut store = Store::new(engine, ());
    let instance = Linker::new(engine)
        .instantiate_async(&mut store, component)
        .await?;
    Ok((store, instance))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let case = std::env::args().nth(1).unwrap_or_default();
    let engine = Engine::new(&Config::new())?;
    let component = Component::new(&engine, GUEST)?;
    let (mut store_a, _instance_a) = instance(&engine, &component).await?;
    let (mut store_b, instance_b) = instance(&engine, &component).await?;

    // Both stores are fresh, so each stream gets the same host-table index and
    // generation in its own store.
    let reader_a = StreamReader::new(&mut store_a, vec![0xaaaa_u32])?;
    let _reader_b = match case.as_str() {
        "mismatch" => {
            let r = StreamReader::new(&mut store_b, vec!["hello".to_string()])?;
            println!("A: {reader_a:?}  B: {r:?} (String)");
        }
        "match" => {
            let r = StreamReader::new(&mut store_b, vec![0xbbbb_u32])?;
            println!("A: {reader_a:?}  B: {r:?} (u32)");
        }
        _ => {
            println!("usage: cross-store-reader-poc <mismatch|match>");
            return Ok(());
        }
    };

    // Lower store A's reader into store B's guest.
    let consume = instance_b
        .get_typed_func::<(StreamReader<u32>,), (u32,)>(&mut store_b, "consume-u")?;
    let r = consume.call_async(&mut store_b, (reader_a,)).await;
    match r {
        Ok((v,)) => println!("{case} -> Ok(0x{v:x})"),
        Err(e) => println!("{case} -> Err({e:?})"),
    }
    Ok(())
}
