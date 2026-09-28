;;! bulk_memory = true
;;! function_references = true
;;! stack_switching = true

;; Spec tests for resume_throw and resume_throw_ref

;; Test resume_throw on a continuation that is never resumed.
(module
  (tag $exn)

  (type $f (func))
  (type $k (cont $f))

  (func $never (unreachable))
  (elem declare func $never)

  (func (export "throw_never")
    (block $h
      (try_table (catch $exn $h)
        (cont.new $k (ref.func $never))
        (resume_throw $k $exn)
        (unreachable)
      )
    )
  )
)
(assert_return (invoke "throw_never"))

;; Test resume_throw with a value type argument.
(module
  (tag $exn_i32 (param i32))

  (type $f (func))
  (type $k (cont $f))

  (func $never (unreachable))
  (elem declare func $never)

  (func (export "throw_never_i32") (result i32)
     (block $h (result i32)
      (try_table (result i32) (catch $exn_i32 $h)
        (i32.const 42)
        (cont.new $k (ref.func $never))
        (resume_throw $k $exn_i32)
        (unreachable)
      )
    )
  )
)
(assert_return (invoke "throw_never_i32") (i32.const 42))

;; Test resume_throw with a reference type argument.
(module
  (tag $exn_ref (param externref))

  (type $f (func))
  (type $k (cont $f))

  (func $never (unreachable))
  (elem declare func $never)

  (func (export "throw_never_ref") (param $val externref) (result externref)
    (block $h (result externref)
      (try_table (result externref) (catch $exn_ref $h)
        (local.get $val)
        (cont.new $k (ref.func $never))
        (resume_throw $k $exn_ref)
        (unreachable)
      )
    )
  )
)
(assert_return (invoke "throw_never_ref" (ref.extern 1)) (ref.extern 1))

;; Test resume_throw where the continuation handles the exception.
(module
  (tag $exn)
  (tag $e1)

  (type $f (func))
  (type $k (cont $f))

  (func $handler
    (block $h
      (try_table (catch $exn $h)
        (suspend $e1)
      )
    )
  )
  (elem declare func $handler)

  (func (export "throw_handled")
    (block $h (result (ref $k))
      (resume $k (on $e1 $h) (cont.new $k (ref.func $handler)))
      (unreachable)
    )
    (resume_throw $k $exn)
  )
)
(assert_return (invoke "throw_handled"))

;; Test resume_throw where the continuation does not handle the exception.
(module
  (tag $exn)
  (tag $e1)

  (type $f (func))
  (type $k (cont $f))

  (func $no_handler
    (suspend $e1)
  )
  (elem declare func $no_handler)

  (func (export "throw_unhandled")
    (block $h (result (ref $k))
      (resume $k (on $e1 $h) (cont.new $k (ref.func $no_handler)))
      (unreachable)
    )
    (resume_throw $k $exn)
  )
)
(assert_exception (invoke "throw_unhandled"))

;; Test resume_throw on a consumed continuation.
(module
  (tag $exn)

  (type $f (func))
  (type $k (cont $f))

  (func $f1)
  (elem declare func $f1)

  (func (export "throw_consumed")
    (local $k_ref (ref $k))
    (local.set $k_ref (cont.new $k (ref.func $f1)))
    (resume $k (local.get $k_ref)) ;; consume it
    (resume_throw $k $exn (local.get $k_ref)) ;; should trap
  )
)
(assert_trap (invoke "throw_consumed") "continuation already consumed")

;; Test resume_throw on a null continuation reference.
(module
  (tag $exn)
  (type $f (func))
  (type $k (cont $f))
  (func (export "throw_null")
    (resume_throw $k $exn (ref.null $k))
  )
)
(assert_trap (invoke "throw_null") "null continuation reference")

;; Test resume_throw_ref where the continuation handles the exception.
(module
  (tag $e0 (param i32))
  (tag $yield)

  (type $f (func (result i32)))
  (type $k (cont $f))

  (func (export "throw_handled_ref") (result i32)
    (local $k_ref (ref $k))
    (local.set $k_ref (cont.new $k (ref.func $yield42)))

    (block $y (result (ref $k))
      (resume $k (on $yield $y)
        (local.get $k_ref))
      (unreachable))
    (local.set $k_ref)

    (block $h (result i32 exnref)
      (try_table (catch_ref $e0 $h)
         (i32.const 42)
	   (throw $e0))
      (unreachable)
    )

    (resume_throw_ref $k (local.get $k_ref))
    (return)
  )

  (func $yield42 (result i32)
    (block $h (result i32)
      (try_table (result i32) (catch $e0 $h)
        (suspend $yield)
	    (unreachable)
      )
    )
  )
  (elem declare func $yield42)
)
(assert_return (invoke "throw_handled_ref") (i32.const 42))


;; Test resume_throw_ref where the continuation does not handle the exception.
(module
  (tag $e0)

  (type $f (func))
  (type $k (cont $f))

  (func $no_handler
    (unreachable) ;; We only throw into this function
  )
  (elem declare func $no_handler)

  (func (export "throw_unhandled_ref")
    (block $h (result exnref)
      (try_table (catch_ref $e0 $h) (throw $e0))
      (unreachable)
    )
    (resume_throw_ref $k (cont.new $k (ref.func $no_handler)))
  )
)
(assert_exception (invoke "throw_unhandled_ref"))

;; Test resume_throw_ref on a consumed continuation.
(module
  (tag $e0)

  (type $f (func))
  (type $k (cont $f))

  (func $f1)
  (elem declare func $f1)

  (func (export "throw_consumed_ref")
    (local $k_ref (ref $k))
    (local.set $k_ref (cont.new $k (ref.func $f1)))
    (resume $k (local.get $k_ref)) ;; consume it

    (block $h (result exnref)
      (try_table (result exnref) (catch_ref $e0 $h)
         (throw $e0)
      )
    )
    (local.get $k_ref)

    (resume_throw_ref $k) ;; should trap
  )
)
(assert_trap (invoke "throw_consumed_ref") "continuation already consumed")

;; Test resume_throw_ref on a null continuation reference.
(module
  (tag $e0)
  (type $f (func))
  (type $k (cont $f))
  (func (export "throw_null_ref")
    (block $h (result exnref)
      (try_table (catch_ref $e0 $h)
         (throw $e0))
      (unreachable)
    )
    (resume_throw_ref $k (ref.null $k))
  )
)
(assert_trap (invoke "throw_null_ref") "null continuation reference")

;; ---- Validation ----

(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (tag $exn (param i32))
    (func
      (i64.const 0)
      (resume_throw $ct $exn (ref.null $ct)) ;; null continuation
      (unreachable)))
  "type mismatch")

(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (tag $exn)
    (func
      (ref.null $ct)
      (i32.const 0)
      (resume_throw $ct $exn) ;; exception tag does not take paramter
      (unreachable)))
  "type mismatch")

(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (tag $exn (param i32))
    (func
      (resume_throw $ct $exn (ref.null $ct)) ;; missing exception payload
      (unreachable)))
  "type mismatch")


(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (tag $exn (param externref))
    (func
      (i64.const 0)
      (resume_throw_ref $ct (ref.null $ct)) ;; expecting an exception ref
      (unreachable)))
  "type mismatch")

(assert_invalid
  (module
    (type $ft (func))
    (type $ct (cont $ft))
    (func
      (resume_throw_ref $ct (ref.null $ct)) ;; expecting an exception ref
      (unreachable)))
  "type mismatch")
