;;! component_model_async = true
;;! component_model_more_async_builtins = true
;;! component_model_threading = true

;; Tests that adding a waitable to a waitable-set traps if that waitable is
;; being used concurrently in a synchronous operation.

(component
  (core module $libc
    (table (export "table") 1 funcref)
    (memory (export "memory") 1)
  )
  (core module $m
    (import "" "thread.new-indirect" (func $thread.new-indirect (param i32 i32) (result i32)))
    (import "" "thread.yield-to-suspended" (func $thread.yield-to-suspended (param i32) (result i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "libc" "table" (table $table 1 funcref))
    (func $thread-run (param $future i32)
      (drop (call $future.read (local.get $future) (i32.const 0)))
    )
    (elem (table $table) (i32.const 0) func $thread-run)
    (func (export "run")
      (local $pair i64)
      (local $future i32)
      (local $set i32)
      (local.set $pair (call $future.new))
      (local.set $future (i32.wrap_i64 (i64.and (local.get $pair) (i64.const 0xFFFFFFFF))))
      (local.set $set (call $waitable-set.new))
      (drop (call $thread.yield-to-suspended (call $thread.new-indirect (i32.const 0) (local.get $future))))
      ;; Should trap, since sync read is still pending:
      (call $waitable.join (local.get $future) (local.get $set))
      unreachable
    )
  )

  (core instance $libc (instantiate $libc))
  (core type $start-func-ty (func (param i32)))
  (alias core export $libc "table" (core table $table))

  (core func $thread.new-indirect (canon thread.new-indirect $start-func-ty (table $table)))
  (core func $thread.yield-to-suspended (canon thread.yield-to-suspended))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (type $future (future))
  (core func $future.new (canon future.new $future))
  (core func $future.read (canon future.read $future (memory $libc "memory")))
  
  (core instance $m (instantiate $m
    (with "" (instance
      (export "thread.new-indirect" (func $thread.new-indirect))
      (export "thread.yield-to-suspended" (func $thread.yield-to-suspended))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "future.new" (func $future.new))
      (export "future.read" (func $future.read))
    ))
    (with "libc" (instance $libc))
  ))

  (func (export "run") async (canon lift (core func $m "run")))
)

(assert_trap (invoke "run") "waitable cannot be used synchronously while added to a waitable set")
