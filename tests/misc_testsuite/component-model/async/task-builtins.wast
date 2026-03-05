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
