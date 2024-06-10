(module $m
  (global (export "g i32") i32 (i32.const 0))
  (global (export "g mut i32") (mut i32)  (i32.const 0))

  (table (export "t funcref") 0 funcref)
  (table (export "t externref") 0 externref)
  (memory (export "mem") 0)

  (func (export "f"))
  (func (export "f p1r2") (param f32) (result i32 i64) unreachable)
)

;; make sure the name of the import is in the message
(assert_unlinkable
  (module (import "m" "g i32" (global i64)))
  "incompatible import type for `m::g i32`")

;; errors on globals
(assert_unlinkable
  (module (import "m" "g i32" (global i64)))
  "expected global of type `i64`, found global of type `i32`")

(assert_unlinkable
  (module (import "m" "g i32" (global (mut i32))))
  "expected mutable global, found immutable global")

(assert_unlinkable
  (module (import "m" "g mut i32" (global i32)))
  "expected immutable global, found mutable global")

;; errors on tables
(assert_unlinkable
  (module (import "m" "t funcref" (table 1 funcref)))
  "expected table limits (min: 1, max: none) doesn't match provided table limits (min: 0, max: none)")

(assert_unlinkable
  (module (import "m" "t externref" (table 0 funcref)))
  "expected table of type `funcref`, found table of type `externref`")

;; errors on memories
(assert_unlinkable
  (module (import "m" "mem" (memory 1)))
  "expected memory limits (min: 1, max: none) doesn't match provided memory limits (min: 0, max: none)")

;; errors on functions
(assert_unlinkable
  (module (import "m" "f" (func (param i32))))
  "expected type `(func (param i32))`, found type `(func)`")

(assert_unlinkable
  (module (import "m" "f p1r2" (func (param i32 i32) (result f64))))
  "expected type `(func (param i32 i32) (result f64))`, found type `(func (param f32) (result i32 i64))`")
