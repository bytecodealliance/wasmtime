;; Test `call_indirect` calls between modules.

(module
  ;; Auxiliary definitions
  (type $proc (func))
  (type $out-i32 (func (result i32)))
  (type $out-i64 (func (result i64)))
  (type $out-f32 (func (result f32)))
  (type $out-f64 (func (result f64)))
  (type $over-i32 (func (param i32) (result i32)))
  (type $over-i64 (func (param i64) (result i64)))
  (type $over-f32 (func (param f32) (result f32)))
  (type $over-f64 (func (param f64) (result f64)))
  (type $f32-i32 (func (param f32 i32) (result i32)))
  (type $i32-i64 (func (param i32 i64) (result i64)))
  (type $f64-f32 (func (param f64 f32) (result f32)))
  (type $i64-f64 (func (param i64 f64) (result f64)))
  (type $over-i32-duplicate (func (param i32) (result i32)))
  (type $over-i64-duplicate (func (param i64) (result i64)))
  (type $over-f32-duplicate (func (param f32) (result f32)))
  (type $over-f64-duplicate (func (param f64) (result f64)))

  (func $const-i32 (type $out-i32) (i32.const 0x132))
  (func $const-i64 (type $out-i64) (i64.const 0x164))
  (func $const-f32 (type $out-f32) (f32.const 0xf32))
  (func $const-f64 (type $out-f64) (f64.const 0xf64))

  (func $id-i32 (type $over-i32) (local.get 0))
  (func $id-i64 (type $over-i64) (local.get 0))
  (func $id-f32 (type $over-f32) (local.get 0))
  (func $id-f64 (type $over-f64) (local.get 0))

  (func $i32-i64 (type $i32-i64) (local.get 1))
  (func $i64-f64 (type $i64-f64) (local.get 1))
  (func $f32-i32 (type $f32-i32) (local.get 1))
  (func $f64-f32 (type $f64-f32) (local.get 1))

  (func $over-i32-duplicate (type $over-i32-duplicate) (local.get 0))
  (func $over-i64-duplicate (type $over-i64-duplicate) (local.get 0))
  (func $over-f32-duplicate (type $over-f32-duplicate) (local.get 0))
  (func $over-f64-duplicate (type $over-f64-duplicate) (local.get 0))

  (table (export "table") funcref
    (elem
      $const-i32 $const-i64 $const-f32 $const-f64
      $id-i32 $id-i64 $id-f32 $id-f64
      $f32-i32 $i32-i64 $f64-f32 $i64-f64
      $over-i32-duplicate $over-i64-duplicate
      $over-f32-duplicate $over-f64-duplicate
    )
  )

  ;; Typing

  (func (export "type-i32") (result i32)
    (call_indirect (type $out-i32) (i32.const 0))
  )
  (func (export "type-i64") (result i64)
    (call_indirect (type $out-i64) (i32.const 1))
  )
  (func (export "type-f32") (result f32)
    (call_indirect (type $out-f32) (i32.const 2))
  )
  (func (export "type-f64") (result f64)
    (call_indirect (type $out-f64) (i32.const 3))
  )

  (func (export "type-index") (result i64)
    (call_indirect (type $over-i64) (i64.const 100) (i32.const 5))
  )

  (func (export "type-first-i32") (result i32)
    (call_indirect (type $over-i32) (i32.const 32) (i32.const 4))
  )
  (func (export "type-first-i64") (result i64)
    (call_indirect (type $over-i64) (i64.const 64) (i32.const 5))
  )
  (func (export "type-first-f32") (result f32)
    (call_indirect (type $over-f32) (f32.const 1.32) (i32.const 6))
  )
  (func (export "type-first-f64") (result f64)
    (call_indirect (type $over-f64) (f64.const 1.64) (i32.const 7))
  )

  (func (export "type-second-i32") (result i32)
    (call_indirect (type $f32-i32) (f32.const 32.1) (i32.const 32) (i32.const 8))
  )
  (func (export "type-second-i64") (result i64)
    (call_indirect (type $i32-i64) (i32.const 32) (i64.const 64) (i32.const 9))
  )
  (func (export "type-second-f32") (result f32)
    (call_indirect (type $f64-f32) (f64.const 64) (f32.const 32) (i32.const 10))
  )
  (func (export "type-second-f64") (result f64)
    (call_indirect (type $i64-f64) (i64.const 64) (f64.const 64.1) (i32.const 11))
  )
)

(register "test")

(module
  ;; Auxiliary definitions. These are the same types as "test"'s, but in a
  ;; different order, since call_indirect types are compared structurally.
  (type $out-i32 (func (result i32)))
  (type $out-i64 (func (result i64)))
  (type $out-f32 (func (result f32)))
  (type $out-f64 (func (result f64)))
  (type $over-i32 (func (param i32) (result i32)))
  (type $over-i64 (func (param i64) (result i64)))
  (type $over-f32 (func (param f32) (result f32)))
  (type $over-f64 (func (param f64) (result f64)))
  (type $f32-i32 (func (param f32 i32) (result i32)))
  (type $i32-i64 (func (param i32 i64) (result i64)))
  (type $f64-f32 (func (param f64 f32) (result f32)))
  (type $i64-f64 (func (param i64 f64) (result f64)))
  (type $over-i32-duplicate (func (param i32) (result i32)))
  (type $over-i64-duplicate (func (param i64) (result i64)))
  (type $over-f32-duplicate (func (param f32) (result f32)))
  (type $over-f64-duplicate (func (param f64) (result f64)))
  (type $proc (func))

  ;; Import the table from "test".

  (import "test" "table" (table 16 funcref))

  ;; Typing

  (func (export "type-i32") (result i32)
    (call_indirect (type $out-i32) (i32.const 0))
  )
  (func (export "type-i64") (result i64)
    (call_indirect (type $out-i64) (i32.const 1))
  )
  (func (export "type-f32") (result f32)
    (call_indirect (type $out-f32) (i32.const 2))
  )
  (func (export "type-f64") (result f64)
    (call_indirect (type $out-f64) (i32.const 3))
  )

  (func (export "type-index") (result i64)
    (call_indirect (type $over-i64) (i64.const 100) (i32.const 5))
  )

  (func (export "type-first-i32") (result i32)
    (call_indirect (type $over-i32) (i32.const 32) (i32.const 4))
  )
  (func (export "type-first-i64") (result i64)
    (call_indirect (type $over-i64) (i64.const 64) (i32.const 5))
  )
  (func (export "type-first-f32") (result f32)
    (call_indirect (type $over-f32) (f32.const 1.32) (i32.const 6))
  )
  (func (export "type-first-f64") (result f64)
    (call_indirect (type $over-f64) (f64.const 1.64) (i32.const 7))
  )

  (func (export "type-second-i32") (result i32)
    (call_indirect (type $f32-i32) (f32.const 32.1) (i32.const 32) (i32.const 8))
  )
  (func (export "type-second-i64") (result i64)
    (call_indirect (type $i32-i64) (i32.const 32) (i64.const 64) (i32.const 9))
  )
  (func (export "type-second-f32") (result f32)
    (call_indirect (type $f64-f32) (f64.const 64) (f32.const 32) (i32.const 10))
  )
  (func (export "type-second-f64") (result f64)
    (call_indirect (type $i64-f64) (i64.const 64) (f64.const 64.1) (i32.const 11))
  )
)

(assert_return (invoke "type-i32") (i32.const 0x132))
(assert_return (invoke "type-i64") (i64.const 0x164))
(assert_return (invoke "type-f32") (f32.const 0xf32))
(assert_return (invoke "type-f64") (f64.const 0xf64))

(assert_return (invoke "type-index") (i64.const 100))

(assert_return (invoke "type-first-i32") (i32.const 32))
(assert_return (invoke "type-first-i64") (i64.const 64))
(assert_return (invoke "type-first-f32") (f32.const 1.32))
(assert_return (invoke "type-first-f64") (f64.const 1.64))

(assert_return (invoke "type-second-i32") (i32.const 32))
(assert_return (invoke "type-second-i64") (i64.const 64))
(assert_return (invoke "type-second-f32") (f32.const 32))
(assert_return (invoke "type-second-f64") (f64.const 64.1))
