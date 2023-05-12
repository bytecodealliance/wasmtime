(module
  (func $f (param i32 i32 i32) (result i32) (local.get 0))
  (func $g (param i32 i32 i32) (result i32) (local.get 1))
  (func $h (param i32 i32 i32) (result i32) (local.get 2))

  ;; Indices:          0  1  2  3  4  5  6  7  8
  (table funcref (elem $f $g $h $f $g $h $f $g $h))
  ;; After table.copy: $g $h $f

  (func (export "copy") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    table.copy)

  (func (export "call") (param i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    call_indirect (param i32 i32 i32) (result i32))
)

;; Call $f at 0
(assert_return
  (invoke "call" (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0))
  (i32.const 1))

;; Call $g at 1
(assert_return
  (invoke "call" (i32.const 0) (i32.const 1) (i32.const 0) (i32.const 1))
  (i32.const 1))

;; Call $h at 2
(assert_return
  (invoke "call" (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 2))
  (i32.const 1))

;; Do a `table.copy` to rearrange the elements. Copy from 4..7 to 0..3.
(invoke "copy" (i32.const 0) (i32.const 4) (i32.const 3))

;; Call $g at 0
(assert_return
  (invoke "call" (i32.const 0) (i32.const 1) (i32.const 0) (i32.const 0))
  (i32.const 1))

;; Call $h at 1
(assert_return
  (invoke "call" (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 1))
  (i32.const 1))

;; Call $f at 2
(assert_return
  (invoke "call" (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 2))
  (i32.const 1))

;; Copying up to the end does not trap.
(invoke "copy" (i32.const 7) (i32.const 0) (i32.const 2))

;; Copying past the end traps.
(assert_trap
  (invoke "copy" (i32.const 7) (i32.const 0) (i32.const 3))
  "undefined element")
