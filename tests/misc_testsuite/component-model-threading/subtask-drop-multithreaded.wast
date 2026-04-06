;;! component_model_async = true
;;! component_model_threading = true
;;! reference_types = true

;; This tests that calling `subtask.drop` on a task which has returned a value
;; and whose main thread has exited but which is still running other threads
;; succeeds without panicking.

(component
  ;; Inner component (callee) with async export
  (component $callee
    (core module $m
      (import "" "task.return" (func $task_return))
      (import "" "thread.new-indirect" (func $thread_new (param i32 i32) (result i32)))
      (import "" "thread.yield" (func $thread_yield (result i32)))
      (import "" "table" (table $table 1 funcref))

      ;; Long-running thread entry: keeps the thread alive
      (func $long_running (param i32)
        ;; Yield repeatedly to stay alive
        (loop $loop
          (drop (call $thread_yield))
          (br $loop)
        )
      )
      (elem (i32.const 0) $long_running)

      ;; The async export
      (func $do_work (export "do-work") (result i32)
        ;; Create an explicit thread that stays alive
        (drop (call $thread_new (i32.const 0) (i32.const 0)))
        ;; Yield before returning to force the caller to wait
        (i32.const 1(;YIELD;))
      )
      
      (func $callback (export "callback") (param i32 i32 i32) (result i32) 
        (call $task_return)
        (i32.const 0(;EXIT;))
      )
    )

    (core module $libc (table (export "table") 1 funcref))
    (core instance $libc (instantiate $libc))
    (core type $start-func-ty (func (param i32)))
    (alias core export $libc "table" (core table $table))
    (core func $thread.new-indirect (canon thread.new-indirect $start-func-ty (table $table)))
    (core func $task.return (canon task.return))
    (core func $thread.yield (canon thread.yield))
    (core instance $m (instantiate $m (with "" (instance
      (export "task.return" (func $task.return))
      (export "thread.new-indirect" (func $thread.new-indirect))
      (export "thread.yield" (func $thread.yield))
      (export "table" (table $table))
    ))))
    (func (export "do-work") async (canon lift (core func $m "do-work") async (callback (func $m "callback"))))
  )

  ;; Outer component (caller)
  (component $caller
    (import "do-work" (func $do-work async))

    (core module $m
      (import "" "do-work" (func $do_work (result i32)))
      (import "" "task.return" (func $task_return))
      (import "" "subtask.drop" (func $subtask_drop (param i32)))
      (import "" "waitable-set.new" (func $set_new (result i32)))
      (import "" "waitable-set.wait" (func $set_wait (param i32 i32) (result i32)))
      (import "" "waitable.join" (func $join (param i32 i32)))
      (import "" "memory" (memory $memory 1))

      (func $run (export "run") (result i32)
        (local $subtask i32)
        (local $set i32)

        (local.set $subtask (i32.shr_u (call $do_work) (i32.const 4)))

        ;; Create a set and join the subtask to it
        (local.set $set (call $set_new))

        (call $join (local.get $subtask) (local.get $set))

        ;; Wait for the Returned event
        (drop (call $set_wait (local.get $set) (i32.const 0)))

        (call $subtask_drop (local.get $subtask))

        (call $task_return)
        
        (i32.const 0(;EXIT;))
      )

      (func $callback (export "callback") (param i32 i32 i32) (result i32)
        unreachable
      )
    )

    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (alias core export $libc "memory" (core memory $memory))
    (canon lower (func $do-work) async (core func $do-work'))
    (core func $task.return (canon task.return))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.wait (canon waitable-set.wait (memory $memory)))
    (core func $waitable.join (canon waitable.join))
    (core func $subtask.drop (canon subtask.drop))
    (core instance $m (instantiate $m (with "" (instance
      (export "task.return" (func $task.return))
      (export "do-work" (func $do-work'))
      (export "subtask.drop" (func $subtask.drop))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "memory" (memory $memory))
    ))))
    (func (export "run") async (canon lift (core func $m "run") async (callback (func $m "callback"))))
  )

  (instance $callee (instantiate $callee))
  (instance $caller (instantiate $caller
    (with "do-work" (func $callee "do-work"))
  ))
                                          
  (func (export "run") (alias export $caller "run"))
)

(assert_return (invoke "run"))

