;; try to create as few 4gb memories as we can to reduce the memory consumption
;; of this test, so create one up front here and use it below.
(module $memory
  (memory (export "memory") i64 0x1_0001 0x1_0005)
)

(module
  (import "memory" "memory" (memory i64 0))
  (func (export "grow") (param i64) (result i64)
    local.get 0
    memory.grow)
  (func (export "size") (result i64)
    memory.size)
)
(assert_return (invoke "grow" (i64.const 0)) (i64.const 0x1_0001))
(assert_return (invoke "size") (i64.const 0x1_0001))

;; TODO: unsure how to test this. Right now growth of any 64-bit memory will
;; always reallocate and copy all the previous memory to a new location, and
;; this means that we're doing a 4gb copy here. That's pretty slow and is just
;; copying a bunch of zeros, so until we optimize that it's not really feasible
;; to test growth in CI andd such.
(;
(assert_return (invoke "grow" (i64.const 1)) (i64.const 0x1_0001))
(assert_return (invoke "size") (i64.const 0x1_0002))
;)

;; Test that initialization with a 64-bit global works
(module $offset
  (global (export "offset") i64 (i64.const 0x1_0000_0000))
)
(module
  (import "offset" "offset" (global i64))
  (import "memory" "memory" (memory i64 0))
  (data (global.get 0) "\01\02\03\04")

  (func (export "load32") (param i64) (result i32)
    local.get 0
    i32.load)
)
(assert_return (invoke "load32" (i64.const 0x1_0000_0000)) (i32.const 0x04030201))

;; Test that initialization with a 64-bit data segment works
(module $offset
  (global (export "offset") i64 (i64.const 0x1_0000_0000))
)
(module
  (import "memory" "memory" (memory i64 0))
  (data (i64.const 0x1_0000_0004) "\01\02\03\04")

  (func (export "load32") (param i64) (result i32)
    local.get 0
    i32.load)
)
(assert_return (invoke "load32" (i64.const 0x1_0000_0004)) (i32.const 0x04030201))

;; loading with a huge offset works
(module $offset
  (global (export "offset") i64 (i64.const 0x1_0000_0000))
)
(module
  (import "memory" "memory" (memory i64 0))
  (data (i64.const 0x1_0000_0004) "\01\02\03\04")

  (func (export "load32") (param i64) (result i32)
    local.get 0
    i32.load offset=0x100000000)
)
(assert_return (invoke "load32" (i64.const 2)) (i32.const 0x02010403))
