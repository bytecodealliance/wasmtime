;;! component_model_async = true
;;! exceptions = true
;;! reference_types = true

;; Tests for the interaction of Wasm exceptions and async lifts/lowers.
;;
;; Unlike the sync->sync case (see `../exceptions.wast`) the fused adapters
;; for calls involving an async lift or lower do not invoke guest code
;; directly: the host drives the callee (and the parameter/result translation
;; functions) itself, so an exception unwinding out of any of that guest code
;; reaches the host, which catches it and reports an error. These tests pin
;; that behavior: if fused-adapter codegen is ever optimized to call guest
;; code directly on these paths, these tests will fail and the adapters
;; involved will need exception barriers like the sync->sync one.
;;
;; In each case the `b` parameter selects when the exception is thrown:
;; `false` throws in the first phase of the call (initial callee invocation)
;; and `true` in a later phase (callback or post-yield).

;; sync->async
(component definition $A
  (component $A
    (core module $a
      (tag $t)
      (func (export "run") (param i32) (result i32)
        local.get 0
        if throw $t end
        i32.const 1 (; CALLBACK_CODE_YIELD ;)
      )
      (func (export "cb") (param i32 i32 i32) (result i32) throw $t)
    )
    (core instance $a (instantiate $a))
    (func (export "run") async (param "b" bool)
      (canon lift (core func $a "run") async (callback (core func $a "cb"))))
  )
  (component $B
    (import "a" (instance $a
      (export "run" (func async (param "b" bool)))
    ))

    (core func $run (canon lower (func $a "run")))
    (core module $b
      (import "" "run" (func $run (param i32)))

      (func (export "run") (param i32)
        block $a
          try_table (catch_all $a)
            (call $run (local.get 0))
            return
          end
        end
        unreachable
      )
    )
    (core instance $b (instantiate $b
      (with "" (instance
        (export "run" (func $run))
      ))
    ))
    (func (export "run") async (param "b" bool) (canon lift (core func $b "run")))
  )
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)

(component instance $A $A)
(assert_trap (invoke "run" (bool.const false)) "thrown Wasm exception")
(component instance $A $A)
(assert_trap (invoke "run" (bool.const true)) "thrown Wasm exception")

;; async->sync
(component definition $A
  (component $A
    (core module $a
      (import "" "yield" (func $yield (result i32)))
      (tag $t)
      (func (export "run") (param i32)
        local.get 0
        if call $yield drop end
        throw $t
      )
    )
    (core func $yield (canon thread.yield))
    (core instance $a (instantiate $a
      (with "" (instance
        (export "yield" (func $yield))
      ))
    ))
    (func (export "run") async (param "b" bool) (canon lift (core func $a "run")))
  )
  (component $B
    (import "a" (instance $a
      (export "run" (func async (param "b" bool)))
    ))

    (core func $run (canon lower (func $a "run") async))

    (core module $libc (memory (export "m") 1))
    (core instance $libc (instantiate $libc))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $waitable-set.wait (canon waitable-set.wait (memory (core memory $libc "m"))))

    (core module $b
      (import "" "run" (func $run (param i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))

      (func (export "run") (param i32)
        (local $subtask i32)
        (local $ws i32)
        block $a
          try_table (catch_all $a)
            (local.set $subtask
              (i32.shr_u (call $run (local.get 0)) (i32.const 4)))
            (local.set $ws (call $waitable-set.new))
            (call $waitable.join (local.get $subtask) (local.get $ws))
            (call $waitable-set.wait (local.get $ws) (i32.const 0))
            unreachable
          end
          unreachable
        end
        unreachable
      )
    )
    (core instance $b (instantiate $b
      (with "" (instance
        (export "run" (func $run))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable.join" (func $waitable.join))
        (export "waitable-set.wait" (func $waitable-set.wait))
      ))
    ))
    (func (export "run") async (param "b" bool) (canon lift (core func $b "run")))
  )
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)

(component instance $A $A)
(assert_trap (invoke "run" (bool.const false)) "thrown Wasm exception")
(component instance $A $A)
(assert_trap (invoke "run" (bool.const true)) "thrown Wasm exception")

;; async->async
(component definition $A
  (component $A
    (core module $a
      (tag $t)
      (func (export "run") (param i32) (result i32)
        local.get 0
        if throw $t end
        i32.const 1 (; CALLBACK_CODE_YIELD ;)
      )
      (func (export "cb") (param i32 i32 i32) (result i32) throw $t)
    )
    (core instance $a (instantiate $a))
    (func (export "run") async (param "b" bool)
      (canon lift (core func $a "run") async (callback (core func $a "cb"))))
  )
  (component $B
    (import "a" (instance $a
      (export "run" (func async (param "b" bool)))
    ))

    (core func $run (canon lower (func $a "run") async))

    (core module $libc (memory (export "m") 1))
    (core instance $libc (instantiate $libc))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $waitable-set.wait (canon waitable-set.wait (memory (core memory $libc "m"))))

    (core module $b
      (import "" "run" (func $run (param i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))

      (func (export "run") (param i32)
        (local $subtask i32)
        (local $ws i32)
        block $a
          try_table (catch_all $a)
            (local.set $subtask
              (i32.shr_u (call $run (local.get 0)) (i32.const 4)))
            (local.set $ws (call $waitable-set.new))
            (call $waitable.join (local.get $subtask) (local.get $ws))
            (call $waitable-set.wait (local.get $ws) (i32.const 0))
            unreachable
          end
          unreachable
        end
        unreachable
      )
    )
    (core instance $b (instantiate $b
      (with "" (instance
        (export "run" (func $run))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable.join" (func $waitable.join))
        (export "waitable-set.wait" (func $waitable-set.wait))
      ))
    ))
    (func (export "run") async (param "b" bool) (canon lift (core func $b "run")))
  )
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)

(component instance $A $A)
(assert_trap (invoke "run" (bool.const false)) "thrown Wasm exception")
(component instance $A $A)
(assert_trap (invoke "run" (bool.const true)) "thrown Wasm exception")
