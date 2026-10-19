mod api;
mod async_;
mod sync;

use wasmtime::Result;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::p2::Interface;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{NamedId, WasiCtx, WasiCtxView, WasiNamedView, WasiView};

/// A component whose labelled `wasi:cli/stdout` import uses `wasi:io` types
/// from labelled imports too, as happens when, for example, dependencies of
/// composed components are isolated from each other. This isn't expressible
/// in WIT.
const IO_NAMED_IMPORTS_COMPONENT: &str = r#"
(component
  (import "e" (implements "wasi:io/error@0.2.12") (instance $e
    (export "error" (type (sub resource)))
  ))
  (alias export $e "error" (type $error))
  (import "p" (implements "wasi:io/poll@0.2.12") (instance $p
    (export "pollable" (type $pollable (sub resource)))
    (export "[method]pollable.block" (func (param "self" (borrow $pollable))))
  ))
  (alias export $p "pollable" (type $pollable))
  (import "s" (implements "wasi:io/streams@0.2.12") (instance $s
    (alias outer 1 $error (type $error'))
    (export "error" (type $error (eq $error')))
    (alias outer 1 $pollable (type $pollable'))
    (export "pollable" (type $pollable (eq $pollable')))
    (export "output-stream" (type $output-stream (sub resource)))
    (type $stream-error' (variant
      (case "last-operation-failed" (own $error))
      (case "closed")
    ))
    (export "stream-error" (type $stream-error (eq $stream-error')))
    (export "[method]output-stream.subscribe"
      (func (param "self" (borrow $output-stream)) (result (own $pollable))))
    (export "[method]output-stream.blocking-write-and-flush"
      (func (param "self" (borrow $output-stream)) (param "contents" (list u8))
        (result (result (error $stream-error)))))
  ))
  (alias export $s "output-stream" (type $output-stream))
  (import "o" (implements "wasi:cli/stdout@0.2.12") (instance $o
    (alias outer 1 $output-stream (type $output-stream'))
    (export "output-stream" (type $output-stream (eq $output-stream')))
    (export "get-stdout" (func (result (own $output-stream))))
  ))

  (alias export $o "get-stdout" (func $get-stdout))
  (alias export $s "[method]output-stream.subscribe" (func $subscribe))
  (alias export $s "[method]output-stream.blocking-write-and-flush" (func $write))
  (alias export $p "[method]pollable.block" (func $block))

  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (alias core export $libc "memory" (core memory $memory))

  (core func $get-stdout (canon lower (func $get-stdout)))
  (core func $subscribe (canon lower (func $subscribe)))
  (core func $write (canon lower (func $write) (memory $memory)))
  (core func $block (canon lower (func $block)))
  (core func $drop-output-stream (canon resource.drop $output-stream))
  (core func $drop-pollable (canon resource.drop $pollable))

  (core module $m
    (import "" "get-stdout" (func $get-stdout (result i32)))
    (import "" "subscribe" (func $subscribe (param i32) (result i32)))
    (import "" "write" (func $write (param i32 i32 i32 i32)))
    (import "" "block" (func $block (param i32)))
    (import "" "drop-output-stream" (func $drop-output-stream (param i32)))
    (import "" "drop-pollable" (func $drop-pollable (param i32)))
    (import "libc" "memory" (memory 1))
    (data (i32.const 0) "hello")

    (func (export "run")
      (local $stdout i32)
      (local $pollable i32)
      (local.set $stdout (call $get-stdout))

      (local.set $pollable (call $subscribe (local.get $stdout)))
      (call $block (local.get $pollable))
      (call $drop-pollable (local.get $pollable))

      (call $write (local.get $stdout) (i32.const 0) (i32.const 5) (i32.const 16))
      (if (i32.load8_u (i32.const 16)) (then unreachable))
      (call $drop-output-stream (local.get $stdout))
    )
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "get-stdout" (func $get-stdout))
      (export "subscribe" (func $subscribe))
      (export "write" (func $write))
      (export "block" (func $block))
      (export "drop-output-stream" (func $drop-output-stream))
      (export "drop-pollable" (func $drop-pollable))
    ))
    (with "libc" (instance $libc))
  ))
  (alias core export $i "run" (core func $run))
  (func (export "run") (canon lift (core func $run)))
)
"#;

/// Store data for running `IO_NAMED_IMPORTS_COMPONENT`, where stdout of the
/// named imports' context is captured.
struct IoNamedImports {
    default: WasiCtx,
    named: WasiCtx,
    table: ResourceTable,
    named_stdout: MemoryOutputPipe,
}

impl Default for IoNamedImports {
    fn default() -> Self {
        let named_stdout = MemoryOutputPipe::new(1024);
        IoNamedImports {
            default: WasiCtx::builder().build(),
            named: WasiCtx::builder().stdout(named_stdout.clone()).build(),
            table: ResourceTable::default(),
            named_stdout,
        }
    }
}

impl IoNamedImports {
    fn lookup(i: Interface, name: &str) -> Result<NamedId> {
        let expected = match name {
            "e" => Interface::IoError,
            "p" => Interface::IoPoll,
            "s" => Interface::IoStreams,
            "o" => Interface::CliStdout,
            _ => panic!("unexpected name: {name}"),
        };
        assert_eq!(i, expected);
        Ok(NamedId(0))
    }
}

impl WasiView for IoNamedImports {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.default,
            table: &mut self.table,
        }
    }
}

impl WasiNamedView for IoNamedImports {
    fn ctx(&mut self, _id: NamedId) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.named,
            table: &mut self.table,
        }
    }
}
