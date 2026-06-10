;;! reference_types = true

;; call_indirect through tables that are never grown, exported, or mutated.
;; Compilation may use a constant bound and elide null/signature checks on
;; these shapes; runtime behavior must be unchanged: in-bounds calls work,
;; and out-of-bounds, null-slot, and signature-mismatch accesses still trap.

;; Mixed-signature immutable table with a null hole.
(module
  (type $i2i (func (param i32) (result i32)))
  (type $v2i (func (result i32)))
  (table 5 funcref)
  (elem (i32.const 0) $add1 $ten $add1)

  (func $add1 (type $i2i) (i32.add (local.get 0) (i32.const 1)))
  (func $ten (type $v2i) (i32.const 10))

  (func (export "call-i2i") (param i32 i32) (result i32)
    (call_indirect (type $i2i) (local.get 1) (local.get 0)))
  (func (export "call-v2i") (param i32) (result i32)
    (call_indirect (type $v2i) (local.get 0))))

(assert_return (invoke "call-i2i" (i32.const 0) (i32.const 41)) (i32.const 42))
(assert_return (invoke "call-i2i" (i32.const 2) (i32.const 7)) (i32.const 8))
(assert_return (invoke "call-v2i" (i32.const 1)) (i32.const 10))

;; Signature mismatch still traps.
(assert_trap (invoke "call-i2i" (i32.const 1) (i32.const 0)) "indirect call type mismatch")
(assert_trap (invoke "call-v2i" (i32.const 0)) "indirect call type mismatch")

;; Null slots still trap: slot 3 was never initialized.
(assert_trap (invoke "call-i2i" (i32.const 3) (i32.const 0)) "uninitialized element")
(assert_trap (invoke "call-v2i" (i32.const 4)) "uninitialized element")

;; Out of bounds still traps against the constant bound.
(assert_trap (invoke "call-i2i" (i32.const 5) (i32.const 0)) "undefined element")
(assert_trap (invoke "call-i2i" (i32.const -1) (i32.const 0)) "undefined element")

;; Uniform-signature immutable table, fully initialized.
(module
  (type $v2i (func (result i32)))
  (table 3 funcref)
  (elem (i32.const 0) $a $b $c)

  (func $a (type $v2i) (i32.const 1))
  (func $b (type $v2i) (i32.const 2))
  (func $c (type $v2i) (i32.const 3))

  (func (export "call") (param i32) (result i32)
    (call_indirect (type $v2i) (local.get 0)))
  (func (export "call-wrong-type") (param i32 i32) (result i32)
    (call_indirect (param i32) (result i32) (local.get 1) (local.get 0))))

(assert_return (invoke "call" (i32.const 0)) (i32.const 1))
(assert_return (invoke "call" (i32.const 1)) (i32.const 2))
(assert_return (invoke "call" (i32.const 2)) (i32.const 3))
(assert_trap (invoke "call" (i32.const 3)) "undefined element")

;; A caller whose expected type differs from the table's uniform type must
;; still observe the mismatch.
(assert_trap (invoke "call-wrong-type" (i32.const 0) (i32.const 0)) "indirect call type mismatch")

;; Same shapes through a declared-growable (no max) table never actually
;; grown: an empty never-grown table has no valid index.
(module
  (table 0 100 funcref)
  (func (export "call-empty") (param i32)
    (call_indirect (local.get 0))))

(assert_trap (invoke "call-empty" (i32.const 0)) "undefined element")
(assert_trap (invoke "call-empty" (i32.const 99)) "undefined element")
