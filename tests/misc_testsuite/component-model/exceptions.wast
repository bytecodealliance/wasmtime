;;! exceptions = true
;;! multi_memory = true
;;! function_references = true
;;! bulk_memory = true
;;! reference_types = true

;; Tests for the interaction of Wasm exceptions and the component model.
;;
;; The component model's canonical ABI specifies that an exception thrown
;; within one component must not propagate to another component; instead it
;; becomes a trap at the component boundary.
;;
;; Each test below links two components: $A exports a function which (or
;; whose canonical-ABI helpers) may throw, and $B calls it across the
;; component boundary from inside a `try_table` whose `catch_all` must never
;; observe an exception from $A. If the exception leaks into $B, its handler
;; executes `unreachable`, producing a distinct trap message from the
;; expected "uncaught exception" trap.

;; An exception thrown by the callee and not caught within $A: the
;; cross-component call must trap rather than unwind into $B.
(component
  (component $A
    (core module $m
      (tag $t)
      (func (export "f") (throw $t)))
    (core instance $i (instantiate $m))
    (func (export "f") (canon lift (core func $i "f"))))

  (component $B
    (import "f" (func $f))
    (core func $f-core (canon lower (func $f)))
    (core module $m
      (import "" "f" (func $f))
      (func (export "run")
        (block $caught
          (try_table (catch_all $caught)
            (call $f))
          (return))
        ;; the exception leaked across the component boundary
        unreachable))
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f-core))))))
    (func (export "run") (canon lift (core func $i "run"))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "run" (func $b "run"))
)
(assert_trap (invoke "run") "uncaught exception propagated out of component")

;; An exception thrown and caught entirely within $A: nothing crosses the
;; component boundary, so the call completes normally.
(component
  (component $A
    (core module $m
      (tag $t)
      (func $throw (throw $t))
      (func (export "f")
        (block $caught
          (try_table (catch_all $caught)
            (call $throw)))))
    (core instance $i (instantiate $m))
    (func (export "f") (canon lift (core func $i "f"))))

  (component $B
    (import "f" (func $f))
    (core func $f-core (canon lower (func $f)))
    (core module $m
      (import "" "f" (func $f))
      (func (export "run")
        (block $caught
          (try_table (catch_all $caught)
            (call $f))
          (return))
        unreachable))
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f-core))))))
    (func (export "run") (canon lift (core func $i "run"))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "run" (func $b "run"))
)
(assert_return (invoke "run"))

;; An exception thrown by the callee's `realloc`, which the adapter invokes
;; to lower the string argument into $A's memory before calling the callee:
;; it likewise must not unwind into $B.
(component
  (component $A
    (core module $m
      (tag $t)
      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (throw $t))
      (func (export "f") (param i32 i32)))
    (core instance $i (instantiate $m))
    (func (export "f") (param "s" string)
      (canon lift (core func $i "f")
        (memory (core memory $i "memory"))
        (realloc (core func $i "realloc")))))

  (component $B
    (import "f" (func $f (param "s" string)))
    (core module $libc
      (memory (export "memory") 1)
      (data (i32.const 16) "hello"))
    (core instance $libc (instantiate $libc))
    (core func $f-core (canon lower (func $f) (memory (core memory $libc "memory"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (func (export "run")
        (block $caught
          (try_table (catch_all $caught)
            (call $f (i32.const 16) (i32.const 5)))
          (return))
        unreachable))
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f-core))))))
    (func (export "run") (canon lift (core func $i "run"))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "run" (func $b "run"))
)
(assert_trap (invoke "run") "uncaught exception propagated out of component")

;; An exception thrown by the callee's `post-return` function, which the
;; adapter invokes after the callee returns: it likewise must not unwind
;; into $B.
(component
  (component $A
    (core module $m
      (tag $t)
      (func (export "f"))
      (func (export "post-f") (throw $t)))
    (core instance $i (instantiate $m))
    (func (export "f")
      (canon lift (core func $i "f") (post-return (core func $i "post-f")))))

  (component $B
    (import "f" (func $f))
    (core func $f-core (canon lower (func $f)))
    (core module $m
      (import "" "f" (func $f))
      (func (export "run")
        (block $caught
          (try_table (catch_all $caught)
            (call $f))
          (return))
        unreachable))
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f-core))))))
    (func (export "run") (canon lift (core func $i "run"))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "run" (func $b "run"))
)
(assert_trap (invoke "run") "uncaught exception propagated out of component")

;; An exception thrown by a resource destructor: component $B drops a
;; resource owned by component $A whose destructor throws. The
;; `resource.drop` intrinsic is likewise a boundary between the two
;; components, so the exception must become a trap rather than unwind into
;; $B.
(component
  (component $A
    ;; The destructor must be supplied when the resource type is declared,
    ;; which is before the module defining the "real" (throwing) destructor
    ;; can be instantiated, since that module imports `resource.new` for the
    ;; type. Route the destructor through a mutable function reference in a
    ;; shim module which is patched after instantiation.
    (core module $shim
      (type $f (func (param i32)))
      (global $dtor-global (export "dtor-global") (mut (ref $f)) (ref.func $dummy))
      (func $dummy (param i32) unreachable)
      (func (export "dtor") (param i32)
        local.get 0
        global.get $dtor-global
        call_ref $f))
    (core instance $shim (instantiate $shim))

    (type $t (resource (rep i32) (dtor (core func $shim "dtor"))))
    (core func $t.new (canon resource.new $t))

    (core module $a
      (import "" "t.new" (func $t.new (param i32) (result i32)))
      (tag $t)
      (func (export "run") (result i32) (call $t.new (i32.const 0)))
      (func (export "dtor-real") (param i32) (throw $t)))
    (core instance $a (instantiate $a
      (with "" (instance (export "t.new" (func $t.new))))))

    (core module $patch
      (type $f (func (param i32)))
      (import "shim" "dtor-global" (global $dtor-global (mut (ref $f))))
      (import "a" "dtor-real" (func $dtor-real (param i32)))
      (func $init (global.set $dtor-global (ref.func $dtor-real)))
      (start $init)
      (elem declare func $dtor-real))
    (core instance (instantiate $patch
      (with "shim" (instance $shim))
      (with "a" (instance $a))))

    (export $t' "t" (type $t))
    (func (export "run") (result (own $t')) (canon lift (core func $a "run"))))

  (component $B
    (import "a" (instance $a
      (export "t" (type $t (sub resource)))
      (export "run" (func (result (own $t))))))

    (core func $run (canon lower (func $a "run")))
    (alias export $a "t" (type $t))
    (core func $t.drop (canon resource.drop $t))
    (core module $b
      (import "" "run" (func $run (result i32)))
      (import "" "t.drop" (func $t.drop (param i32)))
      (func (export "run")
        (block $caught
          (try_table (catch_all $caught)
            (call $t.drop (call $run)))
          (return))
        unreachable))
    (core instance $b (instantiate $b
      (with "" (instance
        (export "run" (func $run))
        (export "t.drop" (func $t.drop))))))
    (func (export "run") (canon lift (core func $b "run"))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)
(assert_trap (invoke "run") "uncaught exception propagated out of component")
