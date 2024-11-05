;;! threads = true

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

(assert_trap (invoke "main") "unaligned atomic")


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

(assert_trap (invoke "wait32") "unaligned atomic")
(assert_trap (invoke "wait64") "unaligned atomic")

(module
  (type (;0;) (func))
  (func $wait32 (type 0)
    i32.const 0
    i32.const 42
    i64.const 0
    memory.atomic.wait32
    unreachable)
  (func $wait64 (type 0)
    i32.const 0
    i64.const 43
    i64.const 0
    memory.atomic.wait64
    unreachable)
  (memory (;0;) 4 4)
  (export "wait32" (func $wait32))
  (export "wait64" (func $wait64))
)

(assert_trap (invoke "wait32") "atomic wait on non-shared memory")
(assert_trap (invoke "wait64") "atomic wait on non-shared memory")

;; not valid values for memory.atomic.wait
(module
  (memory 1 1 shared)
  (type (;0;) (func))
  (func $wait32 (result i32)
    i32.const 0
    i32.const 42
    i64.const -1
    memory.atomic.wait32
    )
  (func $wait64 (result i32)
    i32.const 0
    i64.const 43
    i64.const -1
    memory.atomic.wait64
    )
  (export "wait32" (func $wait32))
  (export "wait64" (func $wait64))
)

(assert_return (invoke "wait32") (i32.const 1))
(assert_return (invoke "wait64") (i32.const 1))

;; timeout
(module
  (memory 1 1 shared)
  (type (;0;) (func))
  (func $wait32 (result i32)
    i32.const 0
    i32.const 0
    i64.const 1000
    memory.atomic.wait32
    )
  (func $wait64 (result i32)
    i32.const 0
    i64.const 0
    i64.const 1000
    memory.atomic.wait64
    )
  (export "wait32" (func $wait32))
  (export "wait64" (func $wait64))
)

(assert_return (invoke "wait32") (i32.const 2))
(assert_return (invoke "wait64") (i32.const 2))

;; timeout on 0ns
(module
  (memory 1 1 shared)
  (type (;0;) (func))
  (func $wait32 (result i32)
    i32.const 0
    i32.const 0
    i64.const 0
    memory.atomic.wait32
    )
  (func $wait64 (result i32)
    i32.const 0
    i64.const 0
    i64.const 0
    memory.atomic.wait64
    )
  (export "wait32" (func $wait32))
  (export "wait64" (func $wait64))
)

(assert_return (invoke "wait32") (i32.const 2))
(assert_return (invoke "wait64") (i32.const 2))
