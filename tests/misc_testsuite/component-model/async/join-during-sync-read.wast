;;! component_model_async = true
;;! component_model_more_async_builtins = true
;;! component_model_threading = true

;; Tests that touching a waitable which another thread is synchronously
;; waiting on traps.

(component
  (core module $libc
    (table (export "table") 1 funcref)
    (memory (export "memory") 1)
  )
  (core module $m
    (import "" "thread.new-indirect" (func $thread.new-indirect (param i32 i32) (result i32)))
    (import "" "thread.yield-then-resume" (func $thread.yield-then-resume (param i32) (result i32)))
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
      (drop (call $thread.yield-then-resume (call $thread.new-indirect (i32.const 0) (local.get $future))))
      ;; Should trap, since sync read is still pending:
      (call $waitable.join (local.get $future) (local.get $set))
      unreachable
    )
  )

  (core instance $libc (instantiate $libc))
  (core type $start-func-ty (func (param i32)))
  (alias core export $libc "table" (core table $table))

  (core func $thread.new-indirect (canon thread.new-indirect $start-func-ty (core table $table)))
  (core func $thread.yield-then-resume (canon thread.yield-then-resume))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (type $future (future))
  (core func $future.new (canon future.new $future))
  (core func $future.read (canon future.read $future (memory (core memory $libc "memory"))))

  (core instance $m (instantiate $m
    (with "" (instance
      (export "thread.new-indirect" (func $thread.new-indirect))
      (export "thread.yield-then-resume" (func $thread.yield-then-resume))
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

(component definition $C
  (core module $libc
    (table (export "table") 4 funcref)
    (memory (export "memory") 1)
  )
  (core module $m
    (import "" "thread.new-indirect" (func $thread.new-indirect (param i32 i32) (result i32)))
    (import "" "thread.yield-then-resume" (func $thread.yield-then-resume (param i32) (result i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
    (import "" "stream.cancel-read" (func $stream.cancel-read (param i32) (result i32)))
    (import "" "stream.cancel-write" (func $stream.cancel-write (param i32) (result i32)))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
    (import "" "future.cancel-read" (func $future.cancel-read (param i32) (result i32)))
    (import "" "future.cancel-write" (func $future.cancel-write (param i32) (result i32)))
    (import "libc" "table" (table $table 4 funcref))

    ;; Thread entry points which block forever in a sync copy, since the other
    ;; end is never used.
    (func $stream-read (param i32)
      (drop (call $stream.read (local.get 0) (i32.const 0) (i32.const 4))))
    (func $stream-write (param i32)
      (drop (call $stream.write (local.get 0) (i32.const 0) (i32.const 4))))
    (func $future-read (param i32)
      (drop (call $future.read (local.get 0) (i32.const 0))))
    (func $future-write (param i32)
      (drop (call $future.write (local.get 0) (i32.const 0))))
    (elem (table $table) (i32.const 0) func $stream-read $stream-write $future-read $future-write)

    ;; Starts a thread running the entry point at table index `$f` on `$end`,
    ;; lets it block, and returns `$end`.
    (func $blocked-in (param $f i32) (param $end i32) (result i32)
      (drop (call $thread.yield-then-resume
        (call $thread.new-indirect (local.get $f) (local.get $end))))
      (local.get $end))

    (func $readable (param i64) (result i32)
      (i32.wrap_i64 (local.get 0)))
    (func $writable (param i64) (result i32)
      (i32.wrap_i64 (i64.shr_u (local.get 0) (i64.const 32))))

    (func (export "waitable.join")
      (call $waitable.join
        (call $blocked-in (i32.const 0) (call $readable (call $stream.new)))
        (i32.const 0))
      unreachable)
    (func (export "stream.cancel-read")
      (drop (call $stream.cancel-read
        (call $blocked-in (i32.const 0) (call $readable (call $stream.new)))))
      unreachable)
    (func (export "stream.cancel-write")
      (drop (call $stream.cancel-write
        (call $blocked-in (i32.const 1) (call $writable (call $stream.new)))))
      unreachable)
    (func (export "future.cancel-read")
      (drop (call $future.cancel-read
        (call $blocked-in (i32.const 2) (call $readable (call $future.new)))))
      unreachable)
    (func (export "future.cancel-write")
      (drop (call $future.cancel-write
        (call $blocked-in (i32.const 3) (call $writable (call $future.new)))))
      unreachable)
  )

  (core instance $libc (instantiate $libc))
  (core type $start-func-ty (func (param i32)))
  (alias core export $libc "table" (core table $table))

  (type $s (stream u8))
  (type $f (future u8))
  (core func $thread.new-indirect (canon thread.new-indirect $start-func-ty (core table $table)))
  (core func $thread.yield-then-resume (canon thread.yield-then-resume))
  (core func $waitable.join (canon waitable.join))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read (canon stream.read $s (memory (core memory $libc "memory"))))
  (core func $stream.write (canon stream.write $s (memory (core memory $libc "memory"))))
  (core func $stream.cancel-read (canon stream.cancel-read $s async))
  (core func $stream.cancel-write (canon stream.cancel-write $s async))
  (core func $future.new (canon future.new $f))
  (core func $future.read (canon future.read $f (memory (core memory $libc "memory"))))
  (core func $future.write (canon future.write $f (memory (core memory $libc "memory"))))
  (core func $future.cancel-read (canon future.cancel-read $f async))
  (core func $future.cancel-write (canon future.cancel-write $f async))

  (core instance $m (instantiate $m
    (with "" (instance
      (export "thread.new-indirect" (func $thread.new-indirect))
      (export "thread.yield-then-resume" (func $thread.yield-then-resume))
      (export "waitable.join" (func $waitable.join))
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
      (export "stream.cancel-read" (func $stream.cancel-read))
      (export "stream.cancel-write" (func $stream.cancel-write))
      (export "future.new" (func $future.new))
      (export "future.read" (func $future.read))
      (export "future.write" (func $future.write))
      (export "future.cancel-read" (func $future.cancel-read))
      (export "future.cancel-write" (func $future.cancel-write))
    ))
    (with "libc" (instance $libc))
  ))

  (func (export "waitable-join") async (canon lift (core func $m "waitable.join")))
  (func (export "stream-cancel-read") async (canon lift (core func $m "stream.cancel-read")))
  (func (export "stream-cancel-write") async (canon lift (core func $m "stream.cancel-write")))
  (func (export "future-cancel-read") async (canon lift (core func $m "future.cancel-read")))
  (func (export "future-cancel-write") async (canon lift (core func $m "future.cancel-write")))
)

(component instance $i $C)
(assert_trap (invoke "waitable-join") "waitable cannot be used synchronously while added to a waitable set")
(component instance $i $C)
(assert_trap (invoke "stream-cancel-read") "waitable cannot be used synchronously while added to a waitable set")
(component instance $i $C)
(assert_trap (invoke "stream-cancel-write") "waitable cannot be used synchronously while added to a waitable set")
(component instance $i $C)
(assert_trap (invoke "future-cancel-read") "waitable cannot be used synchronously while added to a waitable set")
(component instance $i $C)
(assert_trap (invoke "future-cancel-write") "waitable cannot be used synchronously while added to a waitable set")
