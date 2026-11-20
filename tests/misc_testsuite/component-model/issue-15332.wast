;;! exceptions = true
;;! guest_debug = true

;; In each test below component `$B` drops a resource owned by component `$A`,
;; whose destructor runs some code. With guest debugging enabled each store has
;; private copies of code, but the `resource.drop` trampoline that calls the
;; destructor runs out of the engine's copy, so its frame is on the stack while
;; the destructor runs and must be handled by exception unwinding and GC stack
;; walks alike.
;;
;; `$B`'s `run` export wraps the drop in a `catch_all` and executes
;; `unreachable` if it catches anything, so an exception thrown by the
;; destructor must never reach it.

;; An exception thrown by the destructor must be stopped at the
;; `resource.drop` boundary between components and turned into a trap, rather
;; than being caught by the calling component.
(component
  (component $A
    (core module $dtor
      (tag $t)
      (func (export "dtor") (param i32)
        (throw $t)))
    (core instance $dtor (instantiate $dtor))
    (type $t (resource (rep i32) (dtor (core func $dtor "dtor"))))
    (core func $t.new (canon resource.new $t))
    (export $t' "t" (type $t))
    (func (export "new") (param "rep" u32) (result (own $t'))
      (canon lift (core func $t.new))))

  (component $B
    (import "a" (instance $a
      (export "t" (type $t (sub resource)))
      (export "new" (func (param "rep" u32) (result (own $t))))))
    (core func $new (canon lower (func $a "new")))
    (alias export $a "t" (type $t))
    (core func $t.drop (canon resource.drop $t))
    (core module $b
      (import "" "new" (func $new (param i32) (result i32)))
      (import "" "t.drop" (func $t.drop (param i32)))
      (func (export "run")
        (block $caught
          (try_table (catch_all $caught)
            (call $t.drop (call $new (i32.const 0))))
          (return))
        ;; the exception leaked across the component boundary
        unreachable))
    (core instance $b (instantiate $b
      (with "" (instance
        (export "new" (func $new))
        (export "t.drop" (func $t.drop))))))
    (func (export "run") (canon lift (core func $b "run"))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)
(assert_trap (invoke "run") "uncaught exception")

;; A GC while the destructor is running walks the `resource.drop`
;; trampoline's frame.
(component
  (import "wasmtime" (instance $wasmtime
    (export "gc" (func))))

  (component $A
    (import "gc" (func $gc))
    (core func $gc (canon lower (func $gc)))
    (core module $dtor
      (import "" "gc" (func $gc))
      (tag $t)
      (func (export "dtor") (param i32)
        ;; Throw and catch an exception first so that a GC heap exists and
        ;; the GC actually traces roots.
        (block $caught
          (try_table (catch_all $caught)
            (throw $t)))
        (call $gc)))
    (core instance $dtor (instantiate $dtor
      (with "" (instance (export "gc" (func $gc))))))
    (type $t (resource (rep i32) (dtor (core func $dtor "dtor"))))
    (core func $t.new (canon resource.new $t))
    (export $t' "t" (type $t))
    (func (export "new") (param "rep" u32) (result (own $t'))
      (canon lift (core func $t.new))))

  (component $B
    (import "a" (instance $a
      (export "t" (type $t (sub resource)))
      (export "new" (func (param "rep" u32) (result (own $t))))))
    (core func $new (canon lower (func $a "new")))
    (alias export $a "t" (type $t))
    (core func $t.drop (canon resource.drop $t))
    (core module $b
      (import "" "new" (func $new (param i32) (result i32)))
      (import "" "t.drop" (func $t.drop (param i32)))
      (func (export "run")
        (block $caught
          (try_table (catch_all $caught)
            (call $t.drop (call $new (i32.const 0))))
          (return))
        unreachable))
    (core instance $b (instantiate $b
      (with "" (instance
        (export "new" (func $new))
        (export "t.drop" (func $t.drop))))))
    (func (export "run") (canon lift (core func $b "run"))))

  (instance $a (instantiate $A (with "gc" (func $wasmtime "gc"))))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)
(assert_return (invoke "run"))
