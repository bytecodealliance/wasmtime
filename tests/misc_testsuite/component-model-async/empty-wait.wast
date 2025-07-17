;;! component_model_async = true
;;! reference_types = true
;;! gc_types = true

;; This test has two components $C and $D, where $D imports and calls $C
;; $C exports two functions: 'blocker' and 'unblocker'
;;  'blocker' blocks on an empty waitable set
;;  'unblocker' wakes blocker by adding a resolved future to blocker's waitable set
;; $D calls 'blocker' then 'unblocker', then waits for 'blocker' to finish
;;
;; (Copied from
;; https://github.com/WebAssembly/component-model/blob/add-tests/test/concurrency/empty-wait.wast)
(component
  (component $C
    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $CM
      (import "" "mem" (memory 1))
      (import "" "task.return" (func $task.return (param i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (import "" "future.drop-readable" (func $future.drop-readable (param i32)))
      (import "" "future.drop-writable" (func $future.drop-writable (param i32)))

      ;; $ws is waited on by 'blocker' and added to by 'unblocker'
      (global $ws (mut i32) (i32.const 0))
      (func $start (global.set $ws (call $waitable-set.new)))
      (start $start)

      ;; 'unblocker' initializes $futr with the readable end of a resolved future
      (global $futr (mut i32) (i32.const 0))

      (func $blocker (export "blocker") (result i32)
        ;; wait on $ws which is currently empty; 'unblocker' will wake us up
        (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (global.get $ws) (i32.const 4)))
      )
      (func $blocker_cb (export "blocker_cb") (param $event_code i32) (param $index i32) (param $payload i32) (result i32)
        ;; assert that we were in fact woken by 'unblocker' adding $futr to $ws
        (if (i32.ne (i32.const 4 (; FUTURE_READ ;)) (local.get $event_code))
          (then unreachable))
        (if (i32.ne (global.get $futr) (local.get $index))
          (then unreachable))
        (if (i32.ne (i32.const 0) (local.get $payload))
          (then unreachable))

        (call $future.drop-readable (global.get $futr))

        ;; return 42 to $D.run
        (call $task.return (i32.const 42))
        (i32.const 0)
      )

      (func $unblocker (export "unblocker") (result i32)
        (local $ret i32) (local $ret64 i64)
        (local $futw i32)

        ;; create a future that will be used to unblock 'blocker', storing r/w ends in $futr/$futw
        (local.set $ret64 (call $future.new))
        (global.set $futr (i32.wrap_i64 (local.get $ret64)))
        (if (i32.ne (i32.const 2) (global.get $futr))
          (then unreachable))
        (local.set $futw (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))
        (if (i32.ne (i32.const 3) (local.get $futw))
          (then unreachable))

        ;; perform a future.read which will block, and add this future to the waitable-set
        ;; being waited on by 'blocker'
        (local.set $ret (call $future.read (global.get $futr) (i32.const 0)))
        (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
          (then unreachable))
        (call $waitable.join (global.get $futr) (global.get $ws))

        ;; perform a future.write which will rendezvous with the write and complete
        (local.set $ret (call $future.write (local.get $futw) (i32.const 0)))
        (if (i32.ne (i32.const 0) (local.get $ret))
          (then unreachable))

        (call $future.drop-writable (local.get $futw))

        ;; return 43 to $D.run
        (call $task.return (i32.const 43))
        (i32.const 0)
      )
      (func $unblocker_cb (export "unblocker_cb") (param i32 i32 i32) (result i32)
        ;; 'unblocker' doesn't block
        unreachable
      )
    )
    (type $FT (future))
    (canon task.return (result u32) (core func $task.return))
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $memory "mem") (core func $waitable-set.wait))
    (canon future.new $FT (core func $future.new))
    (canon future.read $FT async (core func $future.read))
    (canon future.write $FT async (core func $future.write))
    (canon future.drop-readable $FT (core func $future.drop-readable))
    (canon future.drop-writable $FT (core func $future.drop-writable))
    (core instance $cm (instantiate $CM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "task.return" (func $task.return))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "future.new" (func $future.new))
      (export "future.read" (func $future.read))
      (export "future.write" (func $future.write))
      (export "future.drop-readable" (func $future.drop-readable))
      (export "future.drop-writable" (func $future.drop-writable))
    ))))
    (func (export "blocker") (result u32) (canon lift
      (core func $cm "blocker")
      async (callback (func $cm "blocker_cb"))
    ))
    (func (export "unblocker") (result u32) (canon lift
      (core func $cm "unblocker")
      async (callback (func $cm "unblocker_cb"))
    ))
  )

  (component $D
    (import "c" (instance $c
      (export "blocker" (func (result u32)))
      (export "unblocker" (func (result u32)))
    ))

    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $DM
      (import "" "mem" (memory 1))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "blocker" (func $blocker (param i32) (result i32)))
      (import "" "unblocker" (func $unblocker (param i32) (result i32)))

      (global $ws (mut i32) (i32.const 0))
      (func $start (global.set $ws (call $waitable-set.new)))
      (start $start)

      (func $run (export "run") (result i32)
        (local $ret i32) (local $retp1 i32) (local $retp2 i32)
        (local $subtask i32)
        (local $event_code i32)

        ;; call 'blocker'; it should block
        (local.set $retp1 (i32.const 4))
        (local.set $ret (call $blocker (local.get $retp1)))
        (if (i32.ne (i32.const 1 (; STARTED ;)) (i32.and (local.get $ret) (i32.const 0xf)))
          (then unreachable))
        (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))
        (if (i32.ne (i32.const 2) (local.get $subtask))
          (then unreachable))

        ;; call 'unblocker' to unblock 'blocker'; it should complete eagerly
        (local.set $retp2 (i32.const 8))
        (local.set $ret (call $unblocker (local.get $retp2)))
        (if (i32.ne (i32.const 2 (; RETURNED ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 43) (i32.load (local.get $retp2)))
          (then unreachable))

        ;; wait for 'blocker' to be scheduled, run, and return
        (call $waitable.join (local.get $subtask) (global.get $ws))
        (local.set $retp2 (i32.const 8))
        (local.set $event_code (call $waitable-set.wait (global.get $ws) (local.get $retp2)))
        (if (i32.ne (i32.const 1 (; SUBTASK ;)) (local.get $event_code))
          (then unreachable))
        (if (i32.ne (local.get $subtask) (i32.load (local.get $retp2)))
          (then unreachable))
        (if (i32.ne (i32.const 2 (; RETURNED ;)) (i32.load offset=4 (local.get $retp2)))
          (then unreachable))
        (if (i32.ne (i32.const 42) (i32.load (local.get $retp1)))
          (then unreachable))

        (call $subtask.drop (local.get $subtask))

        ;; return 44 to the top-level test harness
        (i32.const 44)
      )
    )
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $memory "mem") (core func $waitable-set.wait))
    (canon subtask.drop (core func $subtask.drop))
    (canon lower (func $c "blocker") async (memory $memory "mem") (core func $blocker'))
    (canon lower (func $c "unblocker") async (memory $memory "mem") (core func $unblocker'))
    (core instance $dm (instantiate $DM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "subtask.drop" (func $subtask.drop))
      (export "blocker" (func $blocker'))
      (export "unblocker" (func $unblocker'))
    ))))
    (func (export "run") (result u32) (canon lift (core func $dm "run")))
  )

  (instance $c (instantiate $C))
  (instance $d (instantiate $D (with "c" (instance $c))))
  (func (export "run") (alias export $d "run"))
)
(assert_return (invoke "run") (u32.const 44))
