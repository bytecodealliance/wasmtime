;;! component_model_async = true
;;! component_model_async_stackful = true
;;! component_model_async_builtins = true
;;! component_model_threading = true
;;! reference_types = true

;; Tests that cancellation works with the async threading intrinsics.
;; Consists of two components, C and D. C implements functions that mix cancellable and uncancellable yields and suspensions.
;; D calls these functions and cancels the resulting subtasks, ensuring that cancellation is only seen when expected.

;; -- Component C --

;; `run-yield`: Yields twice, first with an uncancellable yield, then with a cancellable yield.
;;      The caller cancels the subtask during the first yield, and ensures that the cancellation only takes effect
;;      on the second yield.

;; `run-yield-to`: Yields twice to a spawned thread, first with an uncancellable yield, then with a cancellable yield.
;;      A complication is that we can't guarantee that if the spawned thread yields, the supertask will be scheduled to
;;      cancel the subtask before the subtask's implicit thread is rescheduled. To handle this, the subtask's implicit
;;      thread first waits on a future to be written by the supertask, then yields to the spawned thread.

;; `run-suspend`: More complex, because executing an uncancellable suspension requires another
;;      thread in the same subtask to explicitly wake it up. This is done by the subtask spawning a new thread that
;;      waits on a future to be written by the supertask, and then resumes the main thread once that happens.
;;      After setting up this thread, `run-suspend` performs an uncancellable suspend, then a cancellable suspend.
;;      The caller cancels the subtask during the first suspend, writes to the future to make the spawned thread
;;      resume the implicit thread, and ensures that the cancellation only takes effect on the second suspend.

;; `run-switch-to`: Similar to `run-suspend`, but uses `thread.switch-to` instead of `thread.suspend`.

;; -- Component D --

;; `run-test`: Calls one of the functions in C based on a test id, cancels the resulting subtask, and ensures that
;;      cancellation is only seen when expected.

;; `run`: Calls `run-test` for each of the functions in C.

(component
    (component $C
        (type $FT (future))
        (core module $Memory (memory (export "mem") 1))
        (core instance $memory (instantiate $Memory))
        ;; Defines the table for the thread start functions, of which there are two
        (core module $libc
            (table (export "__indirect_function_table") 2 funcref))
        (core module $CM
            (import "" "mem" (memory 1))
            (import "" "task.cancel" (func $task-cancel))
            (import "" "thread.new-indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
            (import "" "thread.suspend" (func $thread-suspend (result i32)))
            (import "" "thread.suspend-cancellable" (func $thread-suspend-cancellable (result i32)))
            (import "" "thread.yield-to" (func $thread-yield-to (param i32) (result i32)))
            (import "" "thread.yield-to-cancellable" (func $thread-yield-to-cancellable (param i32) (result i32)))
            (import "" "thread.switch-to" (func $thread-switch-to (param i32) (result i32)))
            (import "" "thread.switch-to-cancellable" (func $thread-switch-to-cancellable (param i32) (result i32)))
            (import "" "thread.yield" (func $thread-yield (result i32)))
            (import "" "thread.yield-cancellable" (func $thread-yield-cancellable (result i32)))
            (import "" "thread.index" (func $thread-index (result i32)))
            (import "" "thread.resume-later" (func $thread-resume-later (param i32)))
            (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
            (import "" "waitable.join" (func $waitable.join (param i32 i32)))
            (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
            (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
            (import "libc" "__indirect_function_table" (table $indirect-function-table 2 funcref))

            ;; Indices into the function table for the thread start functions
            (global $wake-from-suspend-ftbl-idx i32 (i32.const 0))
            (global $just-yield-ftbl-idx i32 (i32.const 1))

            (func (export "run-yield")
                ;; Yield back to the caller, who will attempt to cancel us, but we won't see it
                ;; because we're using an uncancellable yield
                (if (i32.ne (call $thread-yield) (i32.const 0)) (then unreachable))
                ;; Yield back to the caller again. This time, we should receive the cancellation immediately.
                (if (i32.ne (call $thread-yield-cancellable) (i32.const 1)) (then unreachable))
                (call $task-cancel)
            )

            (func $wait-for-future-write (param i32)
                (local $ret i32)
                ;; Perform a future.read, which will block, waiting for the supertask to write
                (local.set $ret (call $future.read (local.get 0) (i32.const 0xba5eba11)))
                (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
                    (then unreachable))
            )

            (func $wake-from-suspend (param i32)
                ;; Extract the thread index and future to wait on from the argument structure
                (local $thread-index i32) (local $future i32)
                (local.set $thread-index (i32.load offset=0 (local.get 0)))
                (local.set $future (i32.load offset=4 (local.get 0)))

                ;; Wait for the supertask to signal us to wake up suspended thread.
                (call $wait-for-future-write (local.get $future))
                ;; Resume the main thread, which is suspended in an uncancellable suspend
                (call $thread-resume-later (local.get $thread-index))
            )

            (func $just-yield (param $explicit-thread-idx i32)
                ;; Yield nondeterministically, either back to the supertask, who will then wait on cancellation to be acknowledged,
                ;; or to the implicit thread, who will acknowledge the cancellation.
                (if (i32.ne (call $thread-yield) (i32.const 0)) (then unreachable))
            )

            ;; Initialize the function table that will be used by thread.new-indirect
            (elem (table $indirect-function-table) (i32.const 0 (; wake-from-suspend-ftbl-idx ;)) func $wake-from-suspend)
            (elem (table $indirect-function-table) (i32.const 1 (; just-yield-ftbl-idx ;)) func $just-yield)

            (func (export "run-yield-to") (param $futr i32)
                (local $thread-index i32)
                ;; Spawn a new thread that will wake us up from our uncancellable suspend; we'll switch to it next
                (local.set $thread-index
                    (call $thread-new-indirect (global.get $just-yield-ftbl-idx) (call $thread-index)))

                ;; We can't guarantee that the supertask will be scheduled to cancel us before we're rescheduled, so we first
                ;; wait on the future to be written, then yield to the spawned thread. This means that cancellation will be
                ;; sent while we're waiting on the future rather than at the yield point, but the cancel will still be pending
                ;; when we reach the yield point, so it should still be ignored by the uncancellable yield and only take effect
                ;; when we reach the second, cancellable yield.
                (call $wait-for-future-write (local.get $futr))

                ;; Yield to the spawned thread uncancellably. We should eventually be rescheduled without being notified
                ;; of the pending cancellation.
                (if (i32.ne (call $thread-yield-to (local.get $thread-index)) (i32.const 0)) (then unreachable))
                ;; Yield to the spawned thread again. This time we should see the cancellation immediately.
                (if (i32.ne (call $thread-yield-to-cancellable (local.get $thread-index)) (i32.const 1)) (then unreachable))
                (call $task-cancel)
            )

            (func (export "run-suspend") (param $futr i32)
                ;; Set up the arguments for the wake-for-suspend thread start function.
                ;; It expects a pointer to a structure containing the thread index to resume
                ;; and the future to wait on before resuming it.
                (local $wake-from-suspend-argp i32)
                (local.set $wake-from-suspend-argp (i32.const 4))
                (i32.store offset=0 (local.get $wake-from-suspend-argp) (call $thread-index))
                (i32.store offset=4 (local.get $wake-from-suspend-argp) (local.get $futr))

                ;; Spawn a new thread that will wake us up from our uncancellable suspend and schedule
                ;; it to resume after we suspend.
                (call $thread-resume-later
                    (call $thread-new-indirect (global.get $wake-from-suspend-ftbl-idx) (local.get $wake-from-suspend-argp)))

                ;; Request suspension. We will not be woken up by cancellation, because this is an uncancellable
                ;; suspend. We will be woken up by the other thread we spawned above, which will be resumed after
                ;; the supertask cancels our subtask.
                (if (i32.ne (call $thread-suspend) (i32.const 0)) (then unreachable))
                ;; Request suspension again. This time we should see the cancellation immediately.
                (if (i32.ne (call $thread-suspend-cancellable) (i32.const 1)) (then unreachable))
                (call $task-cancel)
            )

            (func (export "run-switch-to") (param $futr i32)
                (local $thread-index i32)
                ;; Set up the arguments for the wake-for-suspend thread start function.
                ;; It expects a pointer to a structure containing the thread index to resume
                ;; and the future to wait on before resuming it.
                (local $wake-from-suspend-argp i32)
                (local.set $wake-from-suspend-argp (i32.const 4))
                (i32.store offset=0 (local.get $wake-from-suspend-argp) (call $thread-index))
                (i32.store offset=4 (local.get $wake-from-suspend-argp) (local.get $futr))

                ;; Spawn a new thread that will wake us up from our uncancellable suspend; we'll switch to it next
                (local.set $thread-index
                    (call $thread-new-indirect (global.get $wake-from-suspend-ftbl-idx) (local.get $wake-from-suspend-argp)))

                ;; Request suspension by switching to the spawned thread.
                ;; We will not be woken up by cancellation, because this is an uncancellable suspend.
                ;; We will be woken up by the other thread we spawned above, which will be resumed after
                ;; the supertask cancels our subtask.
                (if (i32.ne (call $thread-switch-to (local.get $thread-index)) (i32.const 0)) (then unreachable))
                ;; Request suspension again. This time we should see the cancellation immediately.
                (if (i32.ne (call $thread-switch-to-cancellable (local.get $thread-index)) (i32.const 1)) (then unreachable))
                (call $task-cancel)
            )
        )

        ;; Instantiate the libc module to get the table
        (core instance $libc (instantiate $libc))
        ;; Get access to `thread.new-indirect` that uses the table from libc
        (core type $start-func-ty (func (param i32)))
        (alias core export $libc "__indirect_function_table" (core table $indirect-function-table))

        (core func $task-cancel (canon task.cancel))
        (core func $thread-new-indirect
            (canon thread.new-indirect $start-func-ty (table $indirect-function-table)))
        (core func $thread-yield (canon thread.yield))
        (core func $thread-yield-cancellable (canon thread.yield cancellable))
        (core func $thread-index (canon thread.index))
        (core func $thread-yield-to (canon thread.yield-to))
        (core func $thread-yield-to-cancellable (canon thread.yield-to cancellable))
        (core func $thread-resume-later (canon thread.resume-later))
        (core func $thread-switch-to (canon thread.switch-to))
        (core func $thread-switch-to-cancellable (canon thread.switch-to cancellable))
        (core func $thread-suspend (canon thread.suspend))
        (core func $thread-suspend-cancellable (canon thread.suspend cancellable))
        (core func $future.read (canon future.read $FT (memory $memory "mem")))
        (core func $waitable-set.new (canon waitable-set.new))
        (core func $waitable.join (canon waitable.join))
        (core func $waitable-set.wait (canon waitable-set.wait (memory $memory "mem")))

        ;; Instantiate the main module
        (core instance $cm (
            instantiate $CM
                (with "" (instance
                    (export "mem" (memory $memory "mem"))
                    (export "task.cancel" (func $task-cancel))
                    (export "thread.new-indirect" (func $thread-new-indirect))
                    (export "thread.index" (func $thread-index))
                    (export "thread.yield-to" (func $thread-yield-to))
                    (export "thread.yield-to-cancellable" (func $thread-yield-to-cancellable))
                    (export "thread.yield" (func $thread-yield))
                    (export "thread.yield-cancellable" (func $thread-yield-cancellable))
                    (export "thread.switch-to" (func $thread-switch-to))
                    (export "thread.switch-to-cancellable" (func $thread-switch-to-cancellable))
                    (export "thread.suspend" (func $thread-suspend))
                    (export "thread.suspend-cancellable" (func $thread-suspend-cancellable))
                    (export "thread.resume-later" (func $thread-resume-later))
                    (export "future.read" (func $future.read))
                    (export "waitable.join" (func $waitable.join))
                    (export "waitable-set.wait" (func $waitable-set.wait))
                    (export "waitable-set.new" (func $waitable-set.new))))
                (with "libc" (instance $libc))))

        (func (export "run-yield") (result u32) (canon lift (core func $cm "run-yield") async))
        (func (export "run-yield-to") async (param "fut" $FT) (result u32) (canon lift (core func $cm "run-yield-to") async))
        (func (export "run-suspend") async (param "fut" $FT) (result u32) (canon lift (core func $cm "run-suspend") async))
        (func (export "run-switch-to") async (param "fut" $FT) (result u32) (canon lift (core func $cm "run-switch-to") async))
    )

    (component $D
        (type $FT (future))
        (import "run-yield" (func $run-yield (result u32)))
        (import "run-yield-to" (func $run-yield-to async (param "fut" $FT) (result u32)))
        (import "run-suspend" (func $run-suspend async (param "fut" $FT) (result u32)))
        (import "run-switch-to" (func $run-switch-to async (param "fut" $FT) (result u32)))

        (core module $Memory (memory (export "mem") 1))
        (core instance $memory (instantiate $Memory))
        (core module $DM
            (import "" "mem" (memory 1))
            (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
            (import "" "run-yield" (func $run-yield (param i32) (result i32)))
            (import "" "run-yield-to" (func $run-yield-to (param i32 i32) (result i32)))
            (import "" "run-suspend" (func $run-suspend (param i32 i32) (result i32)))
            (import "" "run-switch-to" (func $run-switch-to (param i32 i32) (result i32)))
            (import "" "waitable.join" (func $waitable.join (param i32 i32)))
            (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
            (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
            (import "" "future.new" (func $future.new (result i64)))
            (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
            (import "" "thread.yield" (func $thread-yield (result i32)))

            (func $run-test (param $test-id i32) (result i32)
                (local $ret i32) (local $subtask i32)
                (local $ws i32) (local $event_code i32)
                (local $run-retp i32) (local $wait-retp i32)
                (local $ret64 i64) (local $futr i32) (local $futw i32)

                ;; Set up return value storage for run-suspend/switch-to and waitable-set.wait
                (local.set $run-retp (i32.const 4))
                (local.set $wait-retp (i32.const 8))
                (i32.store (local.get $run-retp) (i32.const 0xbad0bad0))
                (i32.store (local.get $wait-retp) (i32.const 0xbad0bad0))

                ;; Create a future that the subtask may wait on
                (local.set $ret64 (call $future.new))
                (local.set $futr (i32.wrap_i64 (local.get $ret64)))
                (local.set $futw (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))

                ;; Calling run-suspend/switch-to will start the thread, which will suspend.
                ;; This is basically a switch statement:
                ;; 0: run-yield
                ;; 1: run-yield-to
                ;; 2: run-suspend
                ;; 3: run-switch-to
                (if (i32.eq (local.get $test-id) (i32.const 0))
                    (then (local.set $ret (call $run-yield (local.get $run-retp))))
                    (else (if (i32.eq (local.get $test-id) (i32.const 1))
                        (then (local.set $ret (call $run-yield-to (local.get $futr) (local.get $run-retp))))
                        (else (if (i32.eq (local.get $test-id) (i32.const 2))
                            (then (local.set $ret (call $run-suspend (local.get $futr) (local.get $run-retp))))
                            (else (if (i32.eq (local.get $test-id) (i32.const 3))
                                (then (local.set $ret (call $run-switch-to (local.get $futr) (local.get $run-retp))))
                                (else unreachable))))))))

                ;; Ensure that the thread started
                (if (i32.ne (i32.and (local.get $ret) (i32.const 0xF)) (i32.const 1 (; STARTED ;)))
                 (then unreachable))
                ;; Extract the subtask index
                (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))
                ;; Cancel the subtask, which should block, because the initial suspend/yield is uncancellable
                (local.set $ret (call $subtask.cancel (local.get $subtask)))
                ;; Ensure the cancellation blocked
                (if (i32.ne (local.get $ret) (i32.const -1 (; BLOCKED ;)))
                 (then unreachable))

                ;; If we're not testing run-yield, the subtask is expecting a write to our future, so write to it
                (if (i32.ne (local.get $test-id) (i32.const 0))
                    (then
                        (local.set $ret (call $future.write (local.get $futw) (i32.const 0xdeadbeef)))
                        ;; The write should succeed
                        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
                            (then unreachable))))

                ;; Wait on the subtask, which will eventually progress to a cancellable yield/suspend and acknowledge the cancellation
                (local.set $ws (call $waitable-set.new))
                (call $waitable.join (local.get $subtask) (local.get $ws))
                (local.set $event_code (call $waitable-set.wait (local.get $ws) (local.get $wait-retp)))
                ;; Ensure we got the subtask event
                (if (i32.ne (local.get $event_code) (i32.const 1 (; SUBTASK ;)))
                 (then unreachable))
                ;; Ensure the subtask index matches
                (if (i32.ne (local.get $subtask) (i32.load (local.get $wait-retp)))
                  (then unreachable))
                ;; Ensure the subtask was cancelled before it returned
                (if (i32.ne (i32.const 4 (; CANCELLED_BEFORE_RETURNED=4 | (0<<4) ;))
                            (i32.load offset=4 (local.get $wait-retp)))
                  (then unreachable))

                ;; Return success
                (i32.const 42)
            )

            (func $run (export "run") (result i32)
                ;; test-id 0: run-yield
                (if (i32.ne (call $run-test (i32.const 0)) (i32.const 42))
                    (then unreachable))

                ;; test-id 1: run-yield-to
                (if (i32.ne (call $run-test (i32.const 1)) (i32.const 42))
                    (then unreachable))

                ;; test-id 2: run-suspend
                (if (i32.ne (call $run-test (i32.const 2)) (i32.const 42))
                    (then unreachable))

                ;; test-id 3: run-switch-to
                (if (i32.ne (call $run-test (i32.const 3)) (i32.const 42))
                    (then unreachable))

                ;; Return success
                (i32.const 42)
            )
        )

        (core func $waitable-set.new (canon waitable-set.new))
        (core func $waitable-set.wait (canon waitable-set.wait (memory $memory "mem")))
        (core func $waitable.join (canon waitable.join))
        (core func $subtask.cancel (canon subtask.cancel async))
        (core func $future.new (canon future.new $FT))
        (core func $future.write (canon future.write $FT (memory $memory "mem")))
        (core func $thread.yield (canon thread.yield))
        (canon lower (func $run-yield) async (memory $memory "mem") (core func $run-yield'))
        (canon lower (func $run-suspend) async (memory $memory "mem") (core func $run-suspend'))
        (canon lower (func $run-switch-to) async (memory $memory "mem") (core func $run-switch-to'))
        (canon lower (func $run-yield-to) async (memory $memory "mem") (core func $run-yield-to'))
        (core instance $dm (instantiate $DM (with "" (instance
            (export "mem" (memory $memory "mem"))
            (export "run-yield" (func $run-yield'))
            (export "run-suspend" (func $run-suspend'))
            (export "run-switch-to" (func $run-switch-to'))
            (export "run-yield-to" (func $run-yield-to'))
            (export "waitable.join" (func $waitable.join))
            (export "waitable-set.new" (func $waitable-set.new))
            (export "waitable-set.wait" (func $waitable-set.wait))
            (export "subtask.cancel" (func $subtask.cancel))
            (export "future.new" (func $future.new))
            (export "future.write" (func $future.write))
            (export "thread.yield" (func $thread.yield))
        ))))
        (func (export "run") async (result u32) (canon lift (core func $dm "run")))
    )

    (instance $c (instantiate $C))
    (instance $d (instantiate $D
        (with "run-yield" (func $c "run-yield"))
        (with "run-yield-to" (func $c "run-yield-to"))
        (with "run-suspend" (func $c "run-suspend"))
        (with "run-switch-to" (func $c "run-switch-to"))
    ))
  (func (export "run") (alias export $d "run"))
)

(assert_return (invoke "run") (u32.const 42))
