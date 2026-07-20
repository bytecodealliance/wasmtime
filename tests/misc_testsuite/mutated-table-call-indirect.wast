;;! reference_types = true
;;! bulk_memory = true

;; Runtime behavior on tables that ARE mutated — the call_indirect
;; optimizations for immutable tables must not fire on any of these.

;; A module that grows its own table: dispatch into the grown region
;; must succeed, and the bound must track the grown size.
(module
  (type $ret-i32 (func (result i32)))
  (func $f42 (result i32) i32.const 42)
  (func $f7 (result i32) i32.const 7)
  (table $t 2 10 funcref)
  (elem (i32.const 0) $f42 $f7)
  (func (export "grow") (result i32)
    (table.grow $t (ref.func $f42) (i32.const 3)))
  (func (export "call") (param i32) (result i32)
    (call_indirect $t (type $ret-i32) (local.get 0))))

(assert_return (invoke "call" (i32.const 0)) (i32.const 42))
(assert_return (invoke "call" (i32.const 1)) (i32.const 7))
;; beyond current size, within declared max: still out of bounds pre-grow
(assert_trap (invoke "call" (i32.const 2)) "undefined element")
(assert_return (invoke "grow") (i32.const 2))
;; the grown region is live and initialized with $f42
(assert_return (invoke "call" (i32.const 2)) (i32.const 42))
(assert_return (invoke "call" (i32.const 4)) (i32.const 42))
;; beyond grown size, within declared max: still traps
(assert_trap (invoke "call" (i32.const 5)) "undefined element")

;; A fully-covered uniform-signature table that the module writes null
;; into: the null check must still be performed after the write.
(module
  (type $ret-i32 (func (result i32)))
  (func $one (result i32) i32.const 1)
  (table $t 2 2 funcref)
  (elem (i32.const 0) $one $one)
  (func (export "set-null")
    (table.set $t (i32.const 1) (ref.null func)))
  (func (export "call") (param i32) (result i32)
    (call_indirect $t (type $ret-i32) (local.get 0))))

(assert_return (invoke "call" (i32.const 1)) (i32.const 1))
(invoke "set-null")
(assert_trap (invoke "call" (i32.const 1)) "uninitialized element")
(assert_return (invoke "call" (i32.const 0)) (i32.const 1))

;; An exported table mutated by ANOTHER module through an import: the
;; exporter's own code never mutates the table, so only the export
;; marking keeps the optimizations off. Constant-index and dynamic
;; dispatch must both observe the writes.
(module $exporter
  (type $ret-i32 (func (result i32)))
  (func $f10 (result i32) i32.const 10)
  (table $t (export "tab") 2 2 funcref)
  (elem (i32.const 0) $f10 $f10)
  (func (export "call0") (result i32)
    (call_indirect $t (type $ret-i32) (i32.const 0)))
  (func (export "call") (param i32) (result i32)
    (call_indirect $t (type $ret-i32) (local.get 0))))

(register "exporter" $exporter)

(module $mutator
  (import "exporter" "tab" (table $t 2 2 funcref))
  (func $f99 (result i32) i32.const 99)
  (func $g (result i64) i64.const 5)
  (elem declare func $f99 $g)
  (func (export "clobber-const-slot")
    (table.set $t (i32.const 0) (ref.func $f99)))
  (func (export "clobber-sig")
    (table.set $t (i32.const 1) (ref.func $g)))
  (func (export "clobber-null")
    (table.set $t (i32.const 0) (ref.null func))))

(assert_return (invoke $exporter "call0") (i32.const 10))
(assert_return (invoke $exporter "call" (i32.const 1)) (i32.const 10))
;; cross-instance write is visible at a constant-index call site
(invoke $mutator "clobber-const-slot")
(assert_return (invoke $exporter "call0") (i32.const 99))
;; cross-instance write of a wrong-signature function still trips the sig check
(invoke $mutator "clobber-sig")
(assert_trap (invoke $exporter "call" (i32.const 1)) "indirect call type mismatch")
;; cross-instance null write still trips the null check
(invoke $mutator "clobber-null")
(assert_trap (invoke $exporter "call0") "uninitialized element")
