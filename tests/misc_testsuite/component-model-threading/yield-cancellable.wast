;;! component_model_async = true
;;! component_model_threading = true

(component
  (core module $libc
    (table (export "t") 1 funcref))
  (core instance $libc (instantiate $libc))
  (core type $start-func-ty (func (param i32)))
  (core func $thread.new-indirect
    (canon thread.new-indirect $start-func-ty (table $libc "t")))
  (core func $thread.unsuspend (canon thread.unsuspend))
  (core func $thread.index (canon thread.index))
  (core func $thread.yield-cancellable (canon thread.yield cancellable))
  (core func $task.return (canon task.return))

  (core module $m
    (import "" "thread.new-indirect" (func $thread.new-indirect (param i32 i32) (result i32)))
    (import "" "thread.unsuspend" (func $thread.unsuspend (param i32)))
    (import "" "thread.index" (func $thread.index (result i32)))
    (import "" "thread.yield-cancellable" (func $thread.yield-cancellable (result i32)))
    (import "" "task.return" (func $task.return))
    (import "" "tbl" (table $tbl 1 funcref))

    ;; entrypoint: create a thread, let it run, then yield ourselves.
    (func (export "run") (result i32)
      (local $tid i32)
      (local.set $tid (call $thread.new-indirect (i32.const 0) (call $thread.index)))
      (call $thread.unsuspend (local.get $tid))
      i32.const 1 ;; CALLBACK_CODE_YIELD
    )

    ;; thread: call `thread.yield-cancellable` and double-check it didn't pick
    ;; up anything
    (func $explicit-start (param $ctx i32)
      (if (call $thread.yield-cancellable)
        (then (unreachable)))
    )
    (elem (table $tbl) (i32.const 0) func $explicit-start)

    ;; finishing up the entrypoint: just return
    (func (export "cb") (param i32 i32 i32) (result i32)
      call $task.return
      i32.const 0 ;; CALLBACK_CODE_EXIT
    )
  )

  (core instance $i (instantiate $m (with "" (instance
    (export "thread.new-indirect" (func $thread.new-indirect))
    (export "thread.unsuspend" (func $thread.unsuspend))
    (export "thread.index" (func $thread.index))
    (export "thread.yield-cancellable" (func $thread.yield-cancellable))
    (export "task.return" (func $task.return))
    (export "tbl" (table $libc "t"))))))

  (func (export "run") async
    (canon lift (core func $i "run") async (callback (func $i "cb"))))
)
(assert_return (invoke "run"))
