;;! component_model_async = true
;;! reference_types = true
;;! gc_types = true

;; This test previously caused Wasmtime to panic while handling a trap due to an
;; improperly disposed fiber.  That bug is fixed now, and this test helps ensure
;; it stays fixed.
;;
;; (Copied from https://github.com/bytecodealliance/wasmtime/issues/11668#issue-3402875697)
(component
  (component $A
    (core module $a
      (func (export "run") (result i32)
        i32.const 1)
      (func (export "run-cb") (param i32 i32 i32) (result i32)
        unreachable)
    )

    (core instance $a (instantiate $a))
    (func (export "run") async
      (canon lift (core func $a "run") async (callback (func $a "run-cb"))))
  )
  (component $B
    (import "a" (instance $a (export "run" (func async))))

    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))

    (core func $run (canon lower (func $a "run") async))
    (core func $new (canon waitable-set.new))
    (core func $join (canon waitable.join))
    (core func $drop (canon waitable-set.drop))
    (core func $wait (canon waitable-set.wait (memory $libc "memory")))

    (core module $b
      (import "" "run" (func $run_a (result i32)))
      (import "" "new" (func $new (result i32)))
      (import "" "join" (func $join (param i32 i32)))
      (import "" "drop" (func $drop (param i32)))
      (import "" "wait" (func $wait (param i32 i32) (result i32)))

      (func (export "run")
        (local $ret i32)
        (local $set i32)

        (local.set $ret (call $run_a))

        ;; make sure it's in the "started" state
        (if (i32.ne (i32.and (local.get $ret) (i32.const 0xf)) (i32.const 1))
          (then (unreachable)))

        ;; extract the waitable handle
        (local.set $ret (i32.shr_u (local.get $ret) (i32.const 4)))

        ;; Make a waitable set and insert our handle into it
        (local.set $set (call $new))
        (call $join (local.get $ret) (local.get $set))

        ;; wait for something to happen filling in memory address 4, but don't
        ;; actually see what happened since this traps right now.
        (call $wait (local.get $set) (i32.const 4))
        drop
      )
    )
    (core instance $b (instantiate $b
      (with "" (instance
        (export "run" (func $run))
        (export "new" (func $new))
        (export "join" (func $join))
        (export "drop" (func $drop))
        (export "wait" (func $wait))
      ))
    ))
    (func (export "run") async
      (canon lift (core func $b "run")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)

(assert_trap (invoke "run") "wasm `unreachable` instruction executed")
