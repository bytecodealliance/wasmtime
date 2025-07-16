;;! component_model_async = true
;;! reference_types = true
;;! gc_types = true

;; This test contains two components $C and $D
;; $D.run drives the test and first calls $C.wait-on-set, which waits on
;; a waitable-set. Then $D.run calls $C.drop-while-waiting which attempts
;; to drop the same waitable-set, which should trap.
;;
;; (Copied from
;; https://github.com/WebAssembly/component-model/blob/add-tests/test/concurrency/drop-waitable-set.wast)
(component
  (component $C
    (core module $Core
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))

      (global $ws (mut i32) (i32.const 0))
      (func $start (global.set $ws (call $waitable-set.new)))
      (start $start)

      (func $wait-on-set (export "wait-on-set") (result i32)
        ;; wait on $ws
        (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (global.get $ws) (i32.const 4)))
      )
      (func $drop-while-waiting (export "drop-while-waiting") (result i32)
        (call $waitable-set.drop (global.get $ws))
        unreachable
      )
      (func $unreachable-cb (export "unreachable-cb") (param i32 i32 i32) (result i32)
        unreachable
      )
    )
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.drop (core func $waitable-set.drop))
    (core instance $core (instantiate $Core (with "" (instance
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.drop" (func $waitable-set.drop))
    ))))
    (func (export "wait-on-set") (canon lift
      (core func $core "wait-on-set")
      async (callback (func $core "unreachable-cb"))
    ))
    (func (export "drop-while-waiting") (canon lift
      (core func $core "drop-while-waiting")
      async (callback (func $core "unreachable-cb"))
    ))
  )

  (component $D
    (import "c" (instance $c
      (export "wait-on-set" (func))
      (export "drop-while-waiting" (func))
    ))

    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $Core
      (import "" "mem" (memory 1))
      (import "" "wait-on-set" (func $wait-on-set (result i32)))
      (import "" "drop-while-waiting" (func $drop-while-waiting))
      (func $run (export "run") (result i32)
        (local $ret i32)

        ;; start an async call to 'wait-on-set' which blocks, waiting on a
        ;; waitable-set.
        (local.set $ret (call $wait-on-set))
        (if (i32.ne (i32.const 0x11) (local.get $ret))
          (then unreachable))

        ;; this call will try to drop the same waitable-set, which should trap.
        (call $drop-while-waiting)
        unreachable
      )
    )
    (canon lower (func $c "wait-on-set") async (memory $memory "mem") (core func $wait-on-set'))
    (canon lower (func $c "drop-while-waiting") (core func $drop-while-waiting'))
    (core instance $core (instantiate $Core (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "wait-on-set" (func $wait-on-set'))
      (export "drop-while-waiting" (func $drop-while-waiting'))
    ))))
    (func (export "run") (result u32) (canon lift (core func $core "run")))
  )

  (instance $c (instantiate $C))
  (instance $d (instantiate $D (with "c" (instance $c))))
  (func (export "run") (alias export $d "run"))
)
(assert_trap (invoke "run") "cannot drop waitable set with waiters")
