;;! exceptions = true
;;! reference_types = true
;;! bulk_memory = true
;;! simd = true

;; Cover several payload shapes: empty, scalar, mixed, vector, and reference
;; values.

(module
  (tag $empty)
  (tag $one (param i32))
  (tag $mixed (param i32 i64 f64))

  (func (export "throw-empty") (throw $empty))
  (func (export "throw-one") (throw $one (i32.const 42)))
  (func (export "throw-mixed") (param i32 i64 f64)
    (throw $mixed (local.get 0) (local.get 1) (local.get 2)))

  ;; `v128` payloads occupy a full slot.
  (tag $vec (param i32 v128 f64))
  (func (export "throw-vec")
    (throw $vec (i32.const 1) (v128.const i64x2 2 3) (f64.const 4))))

(assert_exception (invoke "throw-empty"))
(assert_exception (invoke "throw-one"))
(assert_exception
  (invoke "throw-mixed" (i32.const 1) (i64.const 2) (f64.const 3)))
(assert_exception (invoke "throw-vec"))

;; A reference payload remains live across the exception allocation.
(module
  (tag $eref (param externref))
  (func (export "throw-ref") (param externref)
    (throw $eref (local.get 0))))

(assert_exception (invoke "throw-ref" (ref.extern 5)))

;; A function reference is interned before it is stored in the GC heap.
(module
  (tag $fref (param funcref))
  (func $f)
  (elem declare func $f)
  (func (export "throw-funcref")
    (throw $fref (ref.func $f))))

(assert_exception (invoke "throw-funcref"))

;; A caught payload is delivered to the handler's block.
(module
  (tag $e0 (param i32 i64))

  (func $throw (param i32 i64)
    (throw $e0 (local.get 0) (local.get 1)))

  (func (export "catch") (param i32 i64) (result i32 i64)
    (block $b (result i32 i64)
      (try_table (result i32 i64) (catch $e0 $b)
        (call $throw (local.get 0) (local.get 1))
        (i32.const 42)
        (i64.const 100)))))

(assert_return (invoke "catch" (i32.const 1) (i64.const 2))
  (i32.const 1) (i64.const 2))

;; Fallthrough out of a `try_table` with many results.
(module
  (tag $e (param i32))
  (func $callee (result i32) (i32.const 9))
  (func (export "f") (result i32)
    (block $h (result i32)
      (try_table (result i32 i32 i32 i32 i32 i32) (catch $e $h)
        (i32.const 1)
        (i32.const 2)
        (i32.const 3)
        (i32.const 4)
        (i32.const 5)
        (call $callee))
      drop drop drop drop drop)))

(assert_return (invoke "f") (i32.const 1))

;; Exception caught with a many-value payload.
(module
  (tag $e (param i32 i32 i32 i32 i32 i32))

  (func $throw (result i32 i32 i32 i32 i32 i32)
    (throw $e
      (i32.const 1)
      (i32.const 2)
      (i32.const 3)
      (i32.const 4)
      (i32.const 5)
      (i32.const 6)))

  (func (export "f") (result i32)
    (block $handler (result i32 i32 i32 i32 i32 i32)
      (try_table (result i32 i32 i32 i32 i32 i32) (catch $e $handler)
        (call $throw)))
    drop drop drop drop drop))

(assert_return (invoke "f") (i32.const 1))

;; Both a branch and a fallthrough out of a many-result `try_table`.
(module
  (tag $e (param i32))

  (func (export "f") (param $branch i32) (result i32)
    (block $handler (result i32)
      (try_table (result i32 i32 i32 i32 i32 i32) (catch $e $handler)
        (i32.const 1)
        (i32.const 2)
        (i32.const 3)
        (i32.const 4)
        (i32.const 5)
        (i32.const 6)
        (local.get $branch)
        br_if 0)
      drop drop drop drop drop)))

(assert_return (invoke "f" (i32.const 0)) (i32.const 1))
(assert_return (invoke "f" (i32.const 1)) (i32.const 1))

;; Unreachable fallthrough after an unconditional branch out of a many-result
;; `try_table`.
(module
  (tag $e (param i32))
  (func $callee)

  (func (export "f") (result i32)
    (block $handler (result i32)
      (try_table (result i32 i32 i32 i32 i32 i32) (catch $e $handler)
        (call $callee)
        (i32.const 1)
        (i32.const 2)
        (i32.const 3)
        (i32.const 4)
        (i32.const 5)
        (i32.const 6)
        br 0)
      drop drop drop drop drop)))

(assert_return (invoke "f") (i32.const 1))

;; A `try_table` in unreachable code after a `return`.
(module
  (func (export "f")
    (return)
    (try_table (catch_all 0))))

(assert_return (invoke "f"))

;; Handler lookup across nested and already-exited `try_table` scopes.
(module
  (tag $outer)
  (tag $inner)

  (func $throw_outer
    (throw $outer))

  (func $throw_inner
    (throw $inner))

  ;; While both handlers are active, the inner tag does not match and
  ;; lookup continues to the outer handler.
  (func (export "nested") (result i32)
    (block $outer_handler
      (try_table (catch $outer $outer_handler)
        (block $inner_handler
          (try_table (catch $inner $inner_handler)
            (call $throw_outer)))))
    (i32.const 1))

  ;; After the inner try_table ends, its handler is no longer active.
  ;; The outer catch_all handles the throw instead.
  (func (export "after") (result i32)
    (block $stale_inner_handler
      (block $outer_handler
        (try_table (catch_all $outer_handler)
          (try_table (catch $inner $stale_inner_handler)
            (nop))
          (call $throw_inner)))
      (return (i32.const 1)))
    (i32.const 2)))

(assert_return (invoke "nested") (i32.const 1))
(assert_return (invoke "after") (i32.const 1))

;; An `exnref` local is initialized to null.
(module
  (func (export "run") (result i32)
    (local exnref)
    local.get 0
    ref.is_null))

(assert_return (invoke "run") (i32.const 1))

;; A caught funcref payload is still callable.
(module
  (type $ret_i32 (func (result i32)))
  (tag $e (param funcref i32))
  (table 1 funcref)

  (func $payload (result i32) (i32.const 126))
  (elem declare func $payload)

  (func $throw (param funcref)
    (throw $e (local.get 0) (i32.const 42)))

  (func $catch (param funcref) (result funcref i32)
    (block $handler (result funcref i32)
      (try_table (result funcref i32) (catch $e $handler)
        (call $throw (local.get 0))
        (ref.null func)
        (i32.const 0))))

  ;; Catch a funcref payload, then call it to make sure it's still valid.
  (func (export "run") (result i32 i32)
    (local $f funcref)
    (local $i i32)
    (call $catch (ref.func $payload))
    (local.set $i)
    (local.set $f)
    (table.set (i32.const 0) (local.get $f))
    (call_indirect (type $ret_i32) (i32.const 0))
    (local.get $i)))

(assert_return (invoke "run") (i32.const 126) (i32.const 42))
