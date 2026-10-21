;;! component_model_async = true

;; A pending `future.read` belongs to the instance's handle, not to the task
;; which started it. Here `$B.start-read` starts a read and returns while it's
;; still pending, then `$A` creates a stream, which may reuse the exited task's
;; and thread's slots in the store, and finally `$B.finish` writes the future,
;; which completes the read.
(component
  (component $B
    (type $f (future u8))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $future.new (canon future.new $f))
    (core func $future.read
      (canon future.read $f async (memory (core memory $libc "memory"))))
    (core func $future.write
      (canon future.write $f async (memory (core memory $libc "memory"))))
    (core module $m
      (import "libc" "memory" (memory 1))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (global $writer (mut i32) (i32.const 0))
      (func (export "start-read")
        (local $ends i64)
        (local.set $ends (call $future.new))
        (global.set $writer (i32.wrap_i64 (i64.shr_u (local.get $ends) (i64.const 32))))
        (if (i32.ne (call $future.read (i32.wrap_i64 (local.get $ends)) (i32.const 0x100))
                    (i32.const -1 (; BLOCKED ;)))
          (then unreachable)))
      (func (export "finish")
        (i32.store8 (i32.const 0x200) (i32.const 42))
        (if (i32.ne (call $future.write (global.get $writer) (i32.const 0x200))
                    (i32.const 0 (; COMPLETED ;)))
          (then unreachable))
        (if (i32.ne (i32.load8_u (i32.const 0x100)) (i32.const 42))
          (then unreachable))))
    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "future.new" (func $future.new))
        (export "future.read" (func $future.read))
        (export "future.write" (func $future.write))))))
    (func (export "start-read") (canon lift (core func $i "start-read")))
    (func (export "finish") (canon lift (core func $i "finish"))))
  (instance $b (instantiate $B))

  (component $A
    (import "start-read" (func $start-read))
    (import "finish" (func $finish))
    (type $s (stream u8))
    (core func $start-read (canon lower (func $start-read)))
    (core func $finish (canon lower (func $finish)))
    (core func $stream.new (canon stream.new $s))
    (core module $m
      (import "" "start-read" (func $start-read))
      (import "" "finish" (func $finish))
      (import "" "stream.new" (func $stream.new (result i64)))
      (func (export "run")
        (call $start-read)
        (drop (call $stream.new))
        (call $finish)))
    (core instance $i (instantiate $m (with "" (instance
      (export "start-read" (func $start-read))
      (export "finish" (func $finish))
      (export "stream.new" (func $stream.new))))))
    (func (export "run") (canon lift (core func $i "run"))))
  (instance $a (instantiate $A
    (with "start-read" (func $b "start-read"))
    (with "finish" (func $b "finish"))))
  (export "run" (func $a "run")))

(assert_return (invoke "run"))
