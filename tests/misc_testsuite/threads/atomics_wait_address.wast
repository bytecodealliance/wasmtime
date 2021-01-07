;; From https://bugzilla.mozilla.org/show_bug.cgi?id=1684861.
;;

(module
  (type (;0;) (func))
  (func $main (type 0)
    i32.const -64
    i32.const -63
    memory.atomic.notify offset=1
    unreachable)
  (memory (;0;) 4 4)
  (export "main" (func $main))
)

(assert_trap (invoke "main") "misaligned memory access")


(module
  (type (;0;) (func))
  (func $main (type 0)
    i32.const -64
    i32.const -63
    memory.atomic.notify offset=65536
    unreachable)
  (memory (;0;) 4 4)
  (export "main" (func $main))
)

(assert_trap (invoke "main") "out of bounds memory access")


(module
  (type (;0;) (func))
  (func $wait32 (type 0)
    i32.const -64
    i32.const 42
    i64.const 0
    memory.atomic.wait32 offset=1
    unreachable)
  (func $wait64 (type 0)
    i32.const -64
    i64.const 43
    i64.const 0
    memory.atomic.wait64 offset=3
    unreachable)
  (memory (;0;) 4 4)
  (export "wait32" (func $wait32))
  (export "wait64" (func $wait64))
)

(assert_trap (invoke "wait32") "misaligned memory access")
(assert_trap (invoke "wait64") "misaligned memory access")
