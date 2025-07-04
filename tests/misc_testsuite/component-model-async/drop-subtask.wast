;;! component_model_async = true
;;! reference_types = true
;;! gc_types = true

;; This test contains two components: $Looper and $Caller.
;; $Caller starts an async subtask for $Looper.loop and then drops these
;; subtasks in both allowed and disallowed cases, testing for success and
;; traps.
;;
;; (Copied from
;; https://github.com/WebAssembly/component-model/blob/add-tests/test/concurrency/drop-subtask.wast)
(component
  (component $Looper
    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $CoreLooper
      (import "" "mem" (memory 1))
      (import "" "task.return" (func $task.return))

      (global $done (mut i32) (i32.const 0))

      (func $loop (export "loop") (result i32)
        (i32.const 1 (; YIELD ;))
      )
      (func $loop_cb (export "loop_cb") (param $event_code i32) (param $index i32) (param $payload i32) (result i32)
        ;; confirm that we've received a cancellation request
        (if (i32.ne (local.get $event_code) (i32.const 0 (; NONE ;)))
          (then unreachable))
        (if (i32.ne (local.get $index) (i32.const 0))
          (then unreachable))
        (if (i32.ne (local.get $payload) (i32.const 0))
          (then unreachable))

        (if (i32.eqz (global.get $done))
          (then (return (i32.const 1 (; YIELD ;)))))
        (call $task.return)
        (i32.const 0 (; EXIT ;))
      )

      (func $return (export "return")
        (global.set $done (i32.const 1))
      )
    )
    (canon task.return (core func $task.return))
    (core instance $core_looper (instantiate $CoreLooper (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "task.return" (func $task.return))
    ))))
    (func (export "loop") (canon lift
      (core func $core_looper "loop")
      async (callback (func $core_looper "loop_cb"))
    ))
    (func (export "return") (canon lift
      (core func $core_looper "return")
    ))
  )

  (component $Caller
    (import "looper" (instance $looper
      (export "loop" (func))
      (export "return" (func))
    ))

    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $CoreCaller
      (import "" "mem" (memory 1))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "loop" (func $loop (result i32)))
      (import "" "return" (func $return))

      (func $drop-after-return (export "drop-after-return") (result i32)
        (local $ret i32) (local $ws i32) (local $subtask i32)

        ;; start 'loop'
        (local.set $ret (call $loop))
        (if (i32.ne (i32.const 1 (; STARTED ;)) (i32.and (local.get $ret) (i32.const 0xf)))
          (then unreachable))
        (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))

        ;; tell 'loop' to stop
        (call $return)

        ;; wait for 'loop' to run and return
        (local.set $ws (call $waitable-set.new))
        (call $waitable.join (local.get $subtask) (local.get $ws))
        (local.set $ret (call $waitable-set.wait (local.get $ws) (i32.const 0)))
        (if (i32.ne (i32.const 1 (; SUBTASK ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (local.get $subtask) (i32.load (i32.const 0)))
          (then unreachable))
        (if (i32.ne (i32.const 2 (; RETURNED ;)) (i32.load (i32.const 4)))
          (then unreachable))

        ;; ok to drop
        (call $subtask.drop (local.get $subtask))
        (i32.const 42)
      )

      (func $drop-before-return (export "drop-before-return") (result i32)
        (local $ret i32) (local $subtask i32)

        ;; start 'loop'
        (local.set $ret (call $loop (i32.const 0xdead) (i32.const 0xbeef)))
        (if (i32.ne (i32.const 1 (; STARTED ;)) (i32.and (local.get $ret) (i32.const 0xf)))
          (then unreachable))
        (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))

        ;; this should trap
        (call $subtask.drop (local.get $subtask))
        unreachable
      )
    )
    (canon subtask.drop (core func $subtask.drop))
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $memory "mem") (core func $waitable-set.wait))
    (canon lower (func $looper "loop") async (memory $memory "mem") (core func $loop'))
    (canon lower (func $looper "return") (memory $memory "mem") (core func $return'))
    (core instance $core_caller (instantiate $CoreCaller (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "subtask.drop" (func $subtask.drop))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "loop" (func $loop'))
      (export "return" (func $return'))
    ))))
    (func (export "drop-after-return") (result u32) (canon lift
      (core func $core_caller "drop-after-return")
    ))
    (func (export "drop-before-return") (result u32) (canon lift
      (core func $core_caller "drop-before-return")
    ))
  )

  (instance $looper (instantiate $Looper))
  (instance $caller1 (instantiate $Caller (with "looper" (instance $looper))))
  (instance $caller2 (instantiate $Caller (with "looper" (instance $looper))))
  (func (export "drop-after-return") (alias export $caller1 "drop-after-return"))
  (func (export "drop-before-return") (alias export $caller2 "drop-before-return"))
)
(assert_return (invoke "drop-after-return") (u32.const 42))
(assert_trap (invoke "drop-before-return") "cannot drop a subtask which has not yet resolved")
