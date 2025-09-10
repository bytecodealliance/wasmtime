;;! component_model_async = true
;;! component_model_async_builtins = true
;;! reference_types = true
;;! gc_types = true

;; This test contains two components $C and $D where $D imports and calls $C.
;;  $D.run calls $C.f, which blocks on an empty waitable set
;;  $D.run then subtask.cancels $C.f, which resumes $C.f which promptly resolves
;;    without returning a value.
;;
;; (Copied from
;; https://github.com/WebAssembly/component-model/blob/2c17516179f99accdafb01d8e4affcc0f58184cc/test/async/cancel-subtask.wast)
(component
  (component $C
    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $CM
      (import "" "mem" (memory 1))
      (import "" "task.cancel" (func $task.cancel))
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))

      ;; $ws is waited on by 'f'
      (global $ws (mut i32) (i32.const 0))
      (func $start (global.set $ws (call $waitable-set.new)))
      (start $start)

      (func $f (export "f") (result i32)
        ;; wait on $ws which is currently empty, expected to get cancelled
        (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (global.get $ws) (i32.const 4)))
      )
      (func $f_cb (export "f_cb") (param $event_code i32) (param $index i32) (param $payload i32) (result i32)
        ;; confirm that we've received a cancellation request
        (if (i32.ne (local.get $event_code) (i32.const 6 (; TASK_CANCELLED ;)))
          (then unreachable))
        (if (i32.ne (local.get $index) (i32.const 0))
          (then unreachable))
        (if (i32.ne (local.get $payload) (i32.const 0))
          (then unreachable))

        ;; finish without returning a value
        (call $task.cancel)
        (i32.const 0 (; EXIT ;))
      )

      (func $g (export "g") (param $futr i32) (result i32)
        (local $ret i32)
        (local $event_code i32)

        ;; perform a future.read which will block, waiting for the caller to write
        (local.set $ret (call $future.read (local.get $futr) (i32.const 0xdeadbeef)))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))
        (call $waitable.join (local.get $futr) (global.get $ws))

        ;; wait on $ws synchronously, don't expect cancellation
        (local.set $event_code (call $waitable-set.wait (global.get $ws) (i32.const 0)))
        (if (i32.ne (i32.const 4 (; FUTURE_READ ;)) (local.get $event_code))
          (then unreachable))

        ;; finish returning a value
        (i32.const 42)
      )
    )
    (type $FT (future))
    (canon task.cancel (core func $task.cancel))
    (canon future.read $FT async (memory $memory "mem") (core func $future.read))
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $memory "mem") (core func $waitable-set.wait))
    (core instance $cm (instantiate $CM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "task.cancel" (func $task.cancel))
      (export "future.read" (func $future.read))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
    ))))
    (func (export "f") (result u32) (canon lift
      (core func $cm "f")
      async (callback (func $cm "f_cb"))
    ))
    (func (export "g") (param "fut" $FT) (result u32) (canon lift
      (core func $cm "g")
    ))
  )

  (component $D
    (type $FT (future))
    (import "f" (func $f (result u32)))
    (import "g" (func $g (param "fut" $FT) (result u32)))

    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $DM
      (import "" "mem" (memory 1))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "g" (func $g (param i32 i32) (result i32)))

      (func $run (export "run") (result i32)
        (local $ret i32) (local $ret64 i64)
        (local $retp i32) (local $retp1 i32) (local $retp2 i32)
        (local $subtask i32)
        (local $event_code i32)
        (local $futr i32) (local $futw i32)
        (local $ws i32)

        ;; call 'f'; it should block
        (local.set $retp (i32.const 4))
        (i32.store (local.get $retp) (i32.const 0xbad0bad0))
        (local.set $ret (call $f (local.get $retp)))
        (if (i32.ne (i32.const 1 (; STARTED ;)) (i32.and (local.get $ret) (i32.const 0xf)))
          (then unreachable))
        (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))

        ;; cancel 'f'; it should complete without blocking
        (local.set $ret (call $subtask.cancel (local.get $subtask)))
        (if (i32.ne (i32.const 4 (; CANCELLED_BEFORE_RETURNED ;)) (local.get $ret))
          (then unreachable))

        ;; The $retp memory shouldn't have changed
        (if (i32.ne (i32.load (local.get $retp)) (i32.const 0xbad0bad0))
          (then unreachable))

        (call $subtask.drop (local.get $subtask))

        ;; create future that g will wait on
        (local.set $ret64 (call $future.new))
        (local.set $futr (i32.wrap_i64 (local.get $ret64)))
        (local.set $futw (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))

        ;; call 'g'; it should block
        (local.set $retp1 (i32.const 4))
        (local.set $retp2 (i32.const 8))
        (i32.store (local.get $retp1) (i32.const 0xbad0bad0))
        (i32.store (local.get $retp2) (i32.const 0xbad0bad0))
        (local.set $ret (call $g (local.get $futr) (local.get $retp1)))
        (if (i32.ne (i32.const 1 (; STARTED ;)) (i32.and (local.get $ret) (i32.const 0xf)))
          (then unreachable))
        (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))

        ;; cancel 'g'; it should block
        (local.set $ret (call $subtask.cancel (local.get $subtask)))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))

        ;; future.write, unblocking 'g'
        (local.set $ret (call $future.write (local.get $futw) (i32.const 0xdeadbeef)))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
          (then unreachable))

        ;; wait to see 'g' finish and check its return value
        (local.set $ws (call $waitable-set.new))
        (call $waitable.join (local.get $subtask) (local.get $ws))
        (local.set $event_code (call $waitable-set.wait (local.get $ws) (local.get $retp2)))
        (if (i32.ne (i32.const 1 (; SUBTASK ;)) (local.get $event_code))
          (then unreachable))
        (if (i32.ne (local.get $subtask) (i32.load (local.get $retp2)))
          (then unreachable))
        (if (i32.ne (i32.const 2 (; RETURNED=2 | (0<<4) ;)) (i32.load offset=4 (local.get $retp2)))
          (then unreachable))
        (if (i32.ne (i32.const 42) (i32.load (local.get $retp1)))
          (then unreachable))

        ;; return to the top-level assert_return
        (i32.const 42)
      )
    )
    (canon subtask.cancel async (core func $subtask.cancel))
    (canon subtask.drop (core func $subtask.drop))
    (canon future.new $FT (core func $future.new))
    (canon future.write $FT async (memory $memory "mem") (core func $future.write))
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $memory "mem") (core func $waitable-set.wait))
    (canon lower (func $f) async (memory $memory "mem") (core func $f'))
    (canon lower (func $g) async (memory $memory "mem") (core func $g'))
    (core instance $dm (instantiate $DM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "subtask.cancel" (func $subtask.cancel))
      (export "subtask.drop" (func $subtask.drop))
      (export "future.new" (func $future.new))
      (export "future.write" (func $future.write))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "f" (func $f'))
      (export "g" (func $g'))
    ))))
    (func (export "run") (result u32) (canon lift (core func $dm "run")))
  )

  (instance $c (instantiate $C))
  (instance $d (instantiate $D
    (with "f" (func $c "f"))
    (with "g" (func $c "g"))
  ))
  (func (export "run") (alias export $d "run"))
)
(assert_return (invoke "run") (u32.const 42))
