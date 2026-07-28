;;! bulk_memory = true
;;! function_references = true
;;! stack_switching = true

;; Regression test from https://github.com/bytecodealliance/wasmtime/issues/13584
(module
  (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))
  (memory (export "memory") 1)
  (type $ft (func))
  (type $ct (cont $ft))
  (func $body)
  (elem declare func $body)
  (func (export "start") $start (local $k (ref null $ct))
    ref.func $body
    cont.new $ct
    local.set $k
    block $done
      (resume $ct (local.get $k))
      br $done
    end
    i32.const 0
    call $proc_exit)
)
(assert_return (invoke "start"))