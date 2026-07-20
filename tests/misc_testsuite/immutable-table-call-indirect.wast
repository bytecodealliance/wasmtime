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

;; Populated min<max table that nothing grows: the constant bound is the
;; declared min, so indices in [elem-covered, min) hit null slots, and
;; indices in [min, max) are out of bounds even though the type would
;; allow growing that far.
(module
  (type $v2i (func (result i32)))
  (table 16 64 funcref)
  (elem (i32.const 0) $f1 $f2 $f3)

  (func $f1 (type $v2i) (i32.const 1))
  (func $f2 (type $v2i) (i32.const 2))
  (func $f3 (type $v2i) (i32.const 3))

  (func (export "call-minmax") (param i32) (result i32)
    (call_indirect (type $v2i) (local.get 0))))

(assert_return (invoke "call-minmax" (i32.const 0)) (i32.const 1))
(assert_return (invoke "call-minmax" (i32.const 2)) (i32.const 3))
(assert_trap (invoke "call-minmax" (i32.const 3)) "uninitialized element")
(assert_trap (invoke "call-minmax" (i32.const 15)) "uninitialized element")
(assert_trap (invoke "call-minmax" (i32.const 16)) "undefined element")
(assert_trap (invoke "call-minmax" (i32.const 63)) "undefined element")

;; Uniform-signature table WITH a null hole: even if the signature check
;; can be decided at compile time, the null check for the hole cannot.
(module
  (type $v2i (func (result i32)))
  (table 3 3 funcref)
  (elem (i32.const 0) $a)
  (elem (i32.const 2) $a)

  (func $a (type $v2i) (i32.const 7))

  (func (export "call-holey") (param i32) (result i32)
    (call_indirect (type $v2i) (local.get 0))))

(assert_return (invoke "call-holey" (i32.const 0)) (i32.const 7))
(assert_return (invoke "call-holey" (i32.const 2)) (i32.const 7))
(assert_trap (invoke "call-holey" (i32.const 1)) "uninitialized element")

;; Element segments cover only a prefix of the table: the trailing
;; uncovered slots are null and must keep trapping.
(module
  (type $v2i (func (result i32)))
  (table 8 8 funcref)
  (elem (i32.const 0) $a $a $a)

  (func $a (type $v2i) (i32.const 9))

  (func (export "call-prefix") (param i32) (result i32)
    (call_indirect (type $v2i) (local.get 0))))

(assert_return (invoke "call-prefix" (i32.const 1)) (i32.const 9))
(assert_trap (invoke "call-prefix" (i32.const 3)) "uninitialized element")
(assert_trap (invoke "call-prefix" (i32.const 7)) "uninitialized element")
(assert_trap (invoke "call-prefix" (i32.const 8)) "undefined element")

;; Element segments that cannot be applied as a static image (an
;; expressions-form segment here; a dynamic offset is the other case)
;; are deferred to instantiation. The table is still never mutated
;; after instantiation, but its compile-time image is incomplete, so
;; nothing may be decided from the image alone: a deferred entry with a
;; different signature must still trip the signature check.
(module
  (type $v2i (func (result i32)))
  (type $v2l (func (result i64)))
  (table 3 3 funcref)
  (func $a (type $v2i) (i32.const 1))
  (func $b (type $v2l) (i64.const 2))
  (elem (i32.const 0) $a $a)
  (elem (i32.const 2) funcref (ref.func $b))

  (func (export "call-deferred-sig") (param i32) (result i32)
    (call_indirect (type $v2i) (local.get 0))))

(assert_return (invoke "call-deferred-sig" (i32.const 0)) (i32.const 1))
(assert_return (invoke "call-deferred-sig" (i32.const 1)) (i32.const 1))
(assert_trap (invoke "call-deferred-sig" (i32.const 2)) "indirect call type mismatch")

;; Same deferral, writing null over a slot the static prefix filled:
;; the null check must still fire even though every slot of the
;; compile-time image is non-null.
(module
  (type $v2i (func (result i32)))
  (table 2 2 funcref)
  (func $a (type $v2i) (i32.const 5))
  (elem (i32.const 0) $a $a)
  (elem (i32.const 1) funcref (ref.null func))

  (func (export "call-deferred-null") (param i32) (result i32)
    (call_indirect (type $v2i) (local.get 0))))

(assert_return (invoke "call-deferred-null" (i32.const 0)) (i32.const 5))
(assert_trap (invoke "call-deferred-null" (i32.const 1)) "uninitialized element")
