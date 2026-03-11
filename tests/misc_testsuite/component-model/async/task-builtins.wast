;;! component_model_async = true
;;! multi_memory = true
;;! reference_types = true

;; backpressure.inc
(component
  (core module $m
    (import "" "backpressure.inc" (func $backpressure-inc))
  )
  (core func $backpressure-inc (canon backpressure.inc))
  (core instance $i (instantiate $m (with "" (instance (export "backpressure.inc" (func $backpressure-inc))))))
)

;; backpressure.dec
(component
  (core module $m
    (import "" "backpressure.dec" (func $backpressure-dec))
  )
  (core func $backpressure-dec (canon backpressure.dec))
  (core instance $i (instantiate $m (with "" (instance (export "backpressure.dec" (func $backpressure-dec))))))
)

;; task.return
(component
  (core module $m
    (import "" "task.return" (func $task-return (param i32)))
  )
  (core func $task-return (canon task.return (result u32)))
  (core instance $i (instantiate $m (with "" (instance (export "task.return" (func $task-return))))))
)

;; waitable-set.wait
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "waitable-set.wait" (func $waitable-set-wait (param i32 i32) (result i32)))
  )
  (core func $waitable-set-wait (canon waitable-set.wait (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "waitable-set.wait" (func $waitable-set-wait))))))
)

;; waitable-set.poll
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "waitable-set.poll" (func $waitable-set-poll (param i32 i32) (result i32)))
  )
  (core func $waitable-set-poll (canon waitable-set.poll (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "waitable-set.poll" (func $waitable-set-poll))))))
)

;; thread.yield
(component
  (core module $m
    (import "" "thread.yield" (func $thread-yield (result i32)))
  )
  (core func $thread-yield (canon thread.yield))
  (core instance $i (instantiate $m (with "" (instance (export "thread.yield" (func $thread-yield))))))
)

;; subtask.drop
(component
  (core module $m
    (import "" "subtask.drop" (func $subtask-drop (param i32)))
  )
  (core func $subtask-drop (canon subtask.drop))
  (core instance $i (instantiate $m (with "" (instance (export "subtask.drop" (func $subtask-drop))))))
)

;; subtask.cancel
(component
  (core module $m
    (import "" "subtask.cancel" (func $subtask-drop (param i32) (result i32)))
  )
  (core func $subtask-cancel (canon subtask.cancel))
  (core instance $i (instantiate $m (with "" (instance (export "subtask.cancel" (func $subtask-cancel))))))
)

;; Test that some intrinsics are exempt from may-leave checks
(component
  (core func $backpressure.inc (canon backpressure.inc))
  (core func $backpressure.dec (canon backpressure.dec))
  (core func $context.get (canon context.get i32 0))
  (core func $context.set (canon context.set i32 0))

  (core module $DM
    (import "" "backpressure.inc" (func $backpressure.inc))
    (import "" "backpressure.dec" (func $backpressure.dec))
    (import "" "context.get" (func $context.get (result i32)))
    (import "" "context.set" (func $context.set (param i32)))

    (global $g (mut i32) (i32.const 0))

    (func (export "run")
      (call $context.set (i32.const 100))
    )
    (func (export "post-return")
      call $backpressure.inc
      call $backpressure.dec

      ;; context.get should be what was set in `run`
      call $context.get
      i32.const 100
      i32.ne
      if unreachable end

      i32.const 32
      call $context.set
    )
  )
  (core instance $dm (instantiate $DM (with "" (instance
    (export "backpressure.inc" (func $backpressure.inc))
    (export "backpressure.dec" (func $backpressure.dec))
    (export "context.get" (func $context.get))
    (export "context.set" (func $context.set))
  ))))
  (func (export "run")
    (canon lift (core func $dm "run") (post-return (func $dm "post-return"))))
)

(assert_return (invoke "run"))

;; Test calling intrinsics during `realloc` and their values should be set for
;; when main function is called
(component
  (core func $backpressure.inc (canon backpressure.inc))
  (core func $backpressure.dec (canon backpressure.dec))
  (core func $context.get (canon context.get i32 0))
  (core func $context.set (canon context.set i32 0))

  (core module $libc
    (import "" "backpressure.inc" (func $backpressure.inc))
    (import "" "backpressure.dec" (func $backpressure.dec))
    (import "" "context.get" (func $context.get (result i32)))
    (import "" "context.set" (func $context.set (param i32)))

    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (if (i32.ne (local.get 0) (i32.const 0)) (then (unreachable)))
      (if (i32.ne (local.get 1) (i32.const 0)) (then (unreachable)))
      (if (i32.ne (local.get 2) (i32.const 1)) (then (unreachable)))
      (if (i32.ne (local.get 3) (i32.const 2)) (then (unreachable)))

      call $context.get
      i32.const 0
      i32.ne
      if unreachable end

      i32.const 100
      call $context.set

      call $backpressure.inc
      call $backpressure.dec

      i32.const 200
    )
  )

  (core instance $libc (instantiate $libc (with "" (instance
    (export "backpressure.inc" (func $backpressure.inc))
    (export "backpressure.dec" (func $backpressure.dec))
    (export "context.get" (func $context.get))
    (export "context.set" (func $context.set))
  ))))

  (core module $M
    (import "" "backpressure.inc" (func $backpressure.inc))
    (import "" "backpressure.dec" (func $backpressure.dec))
    (import "" "context.get" (func $context.get (result i32)))
    (import "" "context.set" (func $context.set (param i32)))

    (func (export "run") (param i32 i32)
      ;; ptr/len assertion
      (if (i32.ne (local.get 0) (i32.const 200)) (then (unreachable)))
      (if (i32.ne (local.get 1) (i32.const 2)) (then (unreachable)))

      call $backpressure.inc
      call $backpressure.dec

      ;; context.get should be what was set in `realloc`
      (if (i32.ne (call $context.get) (i32.const 100)) (then (unreachable)))
    )
  )
  (core instance $m (instantiate $M (with "" (instance
    (export "backpressure.inc" (func $backpressure.inc))
    (export "backpressure.dec" (func $backpressure.dec))
    (export "context.get" (func $context.get))
    (export "context.set" (func $context.set))
  ))))
  (func (export "run") (param "x" string)
    (canon lift (core func $m "run")
      (realloc (func $libc "realloc"))
      (memory $libc "memory")
    )
  )
)

(assert_return (invoke "run" (str.const "hi")))

;; Test when realloc is called to communicate return values that various
;; intrinsics work and have their expected values.
(component
  (component $A
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))

    (core func $task.return (canon task.return (result string) (memory $libc "memory")))

    (core module $m
      (import "" "task.return" (func $task.return (param i32 i32)))
      (import "" "memory" (memory 1))
      (func (export "run-sync") (result i32)
        i32.const 40
        i32.const 100
        i32.store offset=0

        i32.const 40
        i32.const 2
        i32.store offset=4

        i32.const 40
      )
      (func (export "run-async") (result i32)
        i32.const 100
        i32.const 2
        call $task.return
        i32.const 0 ;; CALLBACK_CODE_EXIT
      )

      (func (export "run-async-cb") (param i32 i32 i32) (result i32) unreachable)
      (data (i32.const 100) "hi")
    )
    (core instance $m (instantiate $m
      (with "" (instance
        (export "task.return" (func $task.return))
        (export "memory" (memory $libc "memory"))
      ))
    ))
    (func (export "run-sync") (result string)
      (canon lift (core func $m "run-sync") (memory $libc "memory"))
    )
    (func (export "run-async") async (result string)
      (canon lift (core func $m "run-async") (memory $libc "memory") async
          (callback (func $m "run-async-cb")))
    )
  )

  (component $B
    (import "a" (instance $a
      (export "run-sync" (func (result string)))
      (export "run-async" (func async (result string)))
    ))

    (core func $backpressure.inc (canon backpressure.inc))
    (core func $backpressure.dec (canon backpressure.dec))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))

    (core module $libc
      (import "" "backpressure.inc" (func $backpressure.inc))
      (import "" "backpressure.dec" (func $backpressure.dec))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))

      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (if (i32.ne (local.get 0) (i32.const 0)) (then (unreachable)))
        (if (i32.ne (local.get 1) (i32.const 0)) (then (unreachable)))
        (if (i32.ne (local.get 2) (i32.const 1)) (then (unreachable)))
        (if (i32.ne (local.get 3) (i32.const 2)) (then (unreachable)))

        (if (i32.ne (call $context.get) (i32.const 400)) (then (unreachable)))
        (call $context.set (i32.const 500))

        call $backpressure.inc
        call $backpressure.dec

        i32.const 200
      )
    )

    (core instance $libc (instantiate $libc (with "" (instance
      (export "backpressure.inc" (func $backpressure.inc))
      (export "backpressure.dec" (func $backpressure.dec))
      (export "context.get" (func $context.get))
      (export "context.set" (func $context.set))
    ))))

    (core func $sync-to-sync
      (canon lower (func $a "run-sync")
        (memory $libc "memory")
        (realloc (func $libc "realloc"))
      )
    )
    (core func $async-to-sync
      (canon lower (func $a "run-sync")
        async
        (memory $libc "memory")
        (realloc (func $libc "realloc"))
      )
    )
    (core func $sync-to-async
      (canon lower (func $a "run-async")
        (memory $libc "memory")
        (realloc (func $libc "realloc"))
      )
    )
    (core func $async-to-async
      (canon lower (func $a "run-async")
        async
        (memory $libc "memory")
        (realloc (func $libc "realloc"))
      )
    )

    (core module $M
      (import "" "context.set" (func $context.set (param i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "sync-to-sync" (func $sync-to-sync (param i32)))
      (import "" "sync-to-async" (func $sync-to-async (param i32)))
      (import "" "async-to-sync" (func $async-to-sync (param i32) (result i32)))
      (import "" "async-to-async" (func $async-to-async (param i32) (result i32)))

      ;; set this tasks's context before calling $run, in calling $run the
      ;; runtime will then call `realloc` above for the string return value
      ;; which should see our 400 value. That will then set 500 which we should
      ;; then see after the return.

      (func (export "sync-to-sync")
        (call $context.set (i32.const 400))
        (call $sync-to-sync (i32.const 20))
        (if (i32.ne (call $context.get) (i32.const 500)) (then (unreachable)))
      )

      (func (export "sync-to-async")
        (call $context.set (i32.const 400))
        (call $sync-to-async (i32.const 20))
        (if (i32.ne (call $context.get) (i32.const 500)) (then (unreachable)))
      )

      (func (export "async-to-sync")
        (call $context.set (i32.const 400))
        (if
          (i32.ne
            (call $async-to-sync (i32.const 20))
            (i32.const 2) ;; RETURNED
          )
          (then (unreachable))
        )
        (if (i32.ne (call $context.get) (i32.const 500)) (then (unreachable)))
      )

      (func (export "async-to-async")
        (call $context.set (i32.const 400))
        (if
          (i32.ne
            (call $async-to-async (i32.const 20))
            (i32.const 2) ;; RETURNED
          )
          (then (unreachable))
        )
        (if (i32.ne (call $context.get) (i32.const 500)) (then (unreachable)))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "context.set" (func $context.set))
      (export "context.get" (func $context.get))
      (export "sync-to-sync" (func $sync-to-sync))
      (export "sync-to-async" (func $sync-to-async))
      (export "async-to-sync" (func $async-to-sync))
      (export "async-to-async" (func $async-to-async))
    ))))
    (func (export "sync-to-sync") async (canon lift (core func $m "sync-to-sync")))
    (func (export "sync-to-async") async (canon lift (core func $m "sync-to-async")))
    (func (export "async-to-sync") async (canon lift (core func $m "async-to-sync")))
    (func (export "async-to-async") async (canon lift (core func $m "async-to-async")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "sync-to-sync" (func $b "sync-to-sync"))
  (export "sync-to-async" (func $b "sync-to-async"))
  (export "async-to-sync" (func $b "async-to-sync"))
  (export "async-to-async" (func $b "async-to-async"))
)

(assert_return (invoke "sync-to-sync"))
(assert_return (invoke "sync-to-async"))
(assert_return (invoke "async-to-sync"))
(assert_return (invoke "async-to-async"))

;; Same as above, but when calling the host.
(component
  (import "host" (instance $host
    (export "return-hi" (func (result string)))
  ))

  (core func $backpressure.inc (canon backpressure.inc))
  (core func $backpressure.dec (canon backpressure.dec))
  (core func $context.get (canon context.get i32 0))
  (core func $context.set (canon context.set i32 0))

  (core module $libc
    (import "" "backpressure.inc" (func $backpressure.inc))
    (import "" "backpressure.dec" (func $backpressure.dec))
    (import "" "context.get" (func $context.get (result i32)))
    (import "" "context.set" (func $context.set (param i32)))

    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (if (i32.ne (local.get 0) (i32.const 0)) (then (unreachable)))
      (if (i32.ne (local.get 1) (i32.const 0)) (then (unreachable)))
      (if (i32.ne (local.get 2) (i32.const 1)) (then (unreachable)))
      (if (i32.ne (local.get 3) (i32.const 2)) (then (unreachable)))

      (if (i32.ne (call $context.get) (i32.const 400)) (then (unreachable)))
      (call $context.set (i32.const 500))

      call $backpressure.inc
      call $backpressure.dec

      i32.const 200
    )
  )

  (core instance $libc (instantiate $libc (with "" (instance
    (export "backpressure.inc" (func $backpressure.inc))
    (export "backpressure.dec" (func $backpressure.dec))
    (export "context.get" (func $context.get))
    (export "context.set" (func $context.set))
  ))))

  (core func $run
    (canon lower (func $host "return-hi")
      (memory $libc "memory")
      (realloc (func $libc "realloc"))
    )
  )

  (core module $M
    (import "" "context.set" (func $context.set (param i32)))
    (import "" "context.get" (func $context.get (result i32)))
    (import "" "run" (func $run (param i32)))

    (func (export "run")
      ;; set this tasks's context before calling $run, in calling $run the
      ;; runtime will then call `realloc` above for the string return value
      ;; which should see our 400 value. That will then set 500 which we
      ;; should then see after the return.
      (call $context.set (i32.const 400))
      (call $run (i32.const 20))
      (if (i32.ne (call $context.get) (i32.const 500)) (then (unreachable)))
    )
  )
  (core instance $m (instantiate $M (with "" (instance
    (export "context.set" (func $context.set))
    (export "context.get" (func $context.get))
    (export "run" (func $run))
  ))))
  (func (export "run") (canon lift (core func $m "run")))
)

(assert_return (invoke "run"))

;; Test when realloc is called to communicate stream/future values that various
;; intrinsics work and have their expected values.
(component
  ;; Component that will drive the reader forward and write to the future/stream
  (component $writer
    (type $FT (future string))
    (type $ST (stream string))
    ;; The reader will provide runner functions for futures and streams separately
    (import "reader" (instance $reader
      (export "run-future" (func async (param "future" $FT) (result u32)))
      (export "run-stream" (func async (param "stream" $ST) (result u32)))
    ))
    (core module $libc
      (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $run-reader-future (canon lower (func $reader "run-future") (memory $libc "memory") async))
    (core func $run-reader-stream (canon lower (func $reader "run-stream") (memory $libc "memory") async))
    (core module $m
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "stream.new" (func $stream.new (result i64)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "task.return" (func $task.return (param i32)))
      (import "" "run-reader-future" (func $run-reader-future (param i32 i32) (result i32)))
      (import "" "run-reader-stream" (func $run-reader-stream (param i32 i32) (result i32)))
      (import "" "memory" (memory 1))

      (global $ws (mut i32) (i32.const 0))
      (global $fw (mut i32) (i32.const 0))
      (global $sw (mut i32) (i32.const 0))
      (global $state (mut i32) (i32.const 0))
      (global $future-subtask (mut i32) (i32.const 0))
      (global $stream-subtask (mut i32) (i32.const 0))

      (func (export "run") (result i32)
        (local $ret i32) (local $ret64 i64)
        (local $fr i32) (local $sr i32)

        ;; store address and length of string
        (i32.store offset=0 (i32.const 40) (i32.const 100))
        (i32.store offset=4 (i32.const 40) (i32.const 2))

        (local.set $ret64 (call $future.new))
        (local.set $fr (i32.wrap_i64 (local.get $ret64)))
        (global.set $fw (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))
        (local.set $ret64 (call $stream.new))
        (local.set $sr (i32.wrap_i64 (local.get $ret64)))
        (global.set $sw (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))

        (local.set $ret (call $run-reader-future (local.get $fr) (i32.const 60)))
        (global.set $future-subtask (i32.shr_u (local.get $ret) (i32.const 4)))
        (local.set $ret (call $future.write (global.get $fw) (i32.const 40)))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
          (then unreachable))

        (local.set $ret (call $run-reader-stream (local.get $sr) (i32.const 60)))
        (global.set $stream-subtask (i32.shr_u (local.get $ret) (i32.const 4)))
        (local.set $ret (call $stream.write (global.get $sw) (i32.const 40) (i32.const 1)))
        (if (i32.ne (i32.const 0x10 (; COMPLETED | 1<<4 ;)) (local.get $ret)) (then (unreachable)))

        ;; Create a waitable set and join both subtasks to wait for both to complete
        (global.set $ws (call $waitable-set.new))
        (call $waitable.join (global.get $stream-subtask) (global.get $ws))
        (call $waitable.join (global.get $future-subtask) (global.get $ws))
        (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (global.get $ws) (i32.const 4)))
      )

      (global $future-completed (mut i32) (i32.const 0))
      (global $stream-completed (mut i32) (i32.const 0))
      
      ;; Callback invoked when a subtask completes. Since we joined both subtasks to the
      ;; same waitable set, this will be called once for each completion. We track which
      ;; subtasks have completed and only return when both are done.
      (func (export "run-cb") (param $event_code i32) (param $index i32) (param $payload i32) (result i32)
        (if (i32.ne (local.get $event_code) (i32.const 1 (; SUBTASK ;))) (then (unreachable)))
        (if (i32.ne (local.get $payload) (i32.const 2 (; RETURNED ;))) (then (unreachable)))
        
        ;; Track which subtask completed
        (if (i32.eq (local.get $index) (global.get $future-subtask))
          (then
            (global.set $future-completed (i32.const 1)))
          (else
            (if (i32.eq (local.get $index) (global.get $stream-subtask))
              (then (global.set $stream-completed (i32.const 1)))
              (else unreachable))))
        
        ;; If both completed, exit; otherwise keep waiting
        (if (result i32) 
          (i32.and (i32.eq (global.get $future-completed) (i32.const 1))
                   (i32.eq (global.get $stream-completed) (i32.const 1)))
            (then
              (call $task.return (i32.const 42))
              (i32.const 0 (; EXIT ;)))
            (else 
              (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (global.get $ws) (i32.const 4)))))
      )

      (data (i32.const 100) "hi")
    )
    (canon future.new $FT (core func $future.new))
    (canon future.write $FT async
      (memory $libc "memory") (core func $future.write))
    (canon stream.new $ST (core func $stream.new))
    (canon stream.write $ST async
      (memory $libc "memory") (core func $stream.write))
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon task.return (result u32) (memory $libc "memory") (core func $task.return))
    (canon context.set i32 0 (core func $context.set))

    (core instance $M (instantiate $m (with "" (instance
      (export "memory" (memory $libc "memory"))
      (export "future.new" (func $future.new))
      (export "future.write" (func $future.write))
      (export "stream.new" (func $stream.new))
      (export "stream.write" (func $stream.write))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "task.return" (func $task.return))
      (export "run-reader-future" (func $run-reader-future))
      (export "run-reader-stream" (func $run-reader-stream))
    ))))

    (func (export "run") async (result u32)
      (canon lift (core func $M "run") (memory $libc "memory")
        async (callback (func $M "run-cb")))
    )
  )

  (component $reader
    (core module $libc
      (import "" "backpressure.inc" (func $backpressure.inc))
      (import "" "backpressure.dec" (func $backpressure.dec))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))

      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (if (i32.ne (local.get 0) (i32.const 0)) (then (unreachable)))
        (if (i32.ne (local.get 1) (i32.const 0)) (then (unreachable)))
        (if (i32.ne (local.get 2) (i32.const 1)) (then (unreachable)))
        (if (i32.ne (local.get 3) (i32.const 2)) (then (unreachable)))

        call $context.get
        i32.const 400
        i32.ne
        if unreachable end
        
        i32.const 500
        call $context.set

        call $backpressure.inc
        call $backpressure.dec

        i32.const 200
      )
    )

    (core func $backpressure.inc (canon backpressure.inc))
    (core func $backpressure.dec (canon backpressure.dec))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))

    (core instance $libc (instantiate $libc (with "" (instance
      (export "backpressure.inc" (func $backpressure.inc))
      (export "backpressure.dec" (func $backpressure.dec))
      (export "context.get" (func $context.get))
      (export "context.set" (func $context.set))
    ))))

    (type $FT (future string))
    (type $ST (stream string))
    (canon future.new $FT (core func $future.new))
    (canon future.read $FT
      (memory $libc "memory") (realloc (func $libc "realloc")) (core func $future.read))
    (canon stream.new $ST (core func $stream.new))
    (canon stream.read $ST
      (memory $libc "memory") (realloc (func $libc "realloc")) (core func $stream.read))
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $libc "memory") (core func $waitable-set.wait))
    (canon task.return (result u32) (memory $libc "memory") (core func $task.return))

    (core module $m
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (import "" "memory" (memory 1))

      ;; Set context[0] to 400, then read the future, which should call realloc and set
      ;; context[0] to 500, then check that we see that value.
      (func (export "run-future") (param $fr i32) (result i32)
        (local $ret i32)

        (call $context.set (i32.const 400))
        (local.set $ret (call $future.read (local.get $fr) (i32.const 40)))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret)) (then (unreachable)))
        (if (i32.ne (call $context.get) (i32.const 500)) (then (unreachable)))
  
        (call $task.return (i32.const 42))
        (i32.const 0 (; EXIT ;))
      )

      ;; Same as above, but for streams.
      (func (export "run-stream") (param $sr i32) (result i32)
        (local $ret i32)

        (call $context.set (i32.const 400))
        (local.set $ret (call $stream.read (local.get $sr) (i32.const 40) (i32.const 1)))
        (if (i32.ne (i32.const 0x10 (; COMPLETED | 1<<4 ;)) (local.get $ret)) (then (unreachable)))
        (if (i32.ne (call $context.get) (i32.const 500)) (then (unreachable)))
        
        (call $task.return (i32.const 42))
        (i32.const 0 (; EXIT ;))
      )

      (func (export "run-cb") (param i32 i32 i32) (result i32) unreachable))

      (core instance $M (instantiate $m (with "" (instance
        (export "future.read" (func $future.read))
        (export "stream.read" (func $stream.read))
        (export "context.set" (func $context.set))
        (export "context.get" (func $context.get))
        (export "task.return" (func $task.return))
        (export "memory" (memory 1))))))
      (func (export "run-future") async (param "future" $FT) (result u32)
        (canon lift (core func $M "run-future") (memory $libc "memory") (realloc (func $libc "realloc"))
          async (callback (func $M "run-cb"))
        )
      )
      (func (export "run-stream") async (param "stream" $ST) (result u32)
        (canon lift (core func $M "run-stream") (memory $libc "memory") (realloc (func $libc "realloc"))
          async (callback (func $M "run-cb"))
        )
      )
  )

  (instance $Reader (instantiate $reader))
  (instance $Writer (instantiate $writer
    (with "reader" (instance $Reader)
  )))

  (func (export "run") (alias export $Writer "run"))
)
(assert_return (invoke "run") (u32.const 42))
