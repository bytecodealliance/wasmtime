;;! component_model_async = true
;;! component_model_threading = true

(component
    (component $C
        (type $FT (future))
        (core module $Memory (memory (export "mem") 1))
        (core instance $memory (instantiate $Memory))
        ;; Defines the table for the thread start function
        (core module $libc
            (table (export "__indirect_function_table") 1 funcref))
        (core module $CM
            ;; Import the threading builtins and the table from libc
            (import "" "mem" (memory 1))
            (import "" "task.cancel" (func $task-cancel))
            (import "" "thread.new_indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
            (import "" "thread.suspend" (func $thread-suspend (result i32)))
            (import "" "thread.suspend-cancellable" (func $thread-suspend-cancellable (result i32)))
            (import "" "thread.yield-to" (func $thread-yield-to (param i32) (result i32)))
            (import "" "thread.switch-to" (func $thread-switch-to (param i32) (result i32)))
            (import "" "thread.yield" (func $thread-yield (result i32)))
            (import "" "thread.yield-cancellable" (func $thread-yield-cancellable (result i32)))
            (import "" "thread.index" (func $thread-index (result i32)))
            (import "" "thread.resume-later" (func $thread-resume-later (param i32)))
            (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
            (import "" "waitable.join" (func $waitable.join (param i32 i32)))
            (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
            (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
            (import "libc" "__indirect_function_table" (table $indirect-function-table 1 funcref))

            (func (export "run-yield")
                ;; Yield back to the caller, who will attempt to cancel us, but we won't see it
                ;; because we're using an uncancellable yield
                (if (i32.ne (call $thread-yield) (i32.const 0)) (then unreachable))
                ;; Yield back to the caller again. This time, we should receive the cancellation immediately.
                (if (i32.ne (call $thread-yield-cancellable) (i32.const 1)) (then unreachable))
                (call $task-cancel)
            )

            (func $wait-for-future-write (param i32)
                ;; Waitable set to wait on the future read
                (local $ws i32) (local $ret i32) (local $event_code i32)
                (local.set $ws (call $waitable-set.new))
                ;; Perform a future.read, which will block, waiting for the supertask to write
                (local.set $ret (call $future.read (local.get 0) (i32.const 0xba5eba11)))
                (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
                    (then unreachable))
                (call $waitable.join (local.get 0) (local.get $ws))

                ;; Wait on $ws synchronously, don't expect cancellation
                (local.set $event_code (call $waitable-set.wait (local.get $ws) (i32.const 0)))
                (if (i32.ne (i32.const 4 (; FUTURE_READ ;)) (local.get $event_code))
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

            ;; Initialize the function table with our wake-from-suspend function; this will be
            ;; used by thread.new_indirect
            (elem (table $indirect-function-table) (i32.const 0) func $wake-from-suspend)

            (func (export "run-suspend") (param i32)
                ;; Set up the arguments for the wake-for-suspend thread start function.
                ;; It expects a pointer to a structure containing the thread index to resume
                ;; and the future to wait on before resuming it.
                (local $wake-from-suspend-argp i32)
                (local.set $wake-from-suspend-argp (i32.const 4))
                (i32.store offset=0 (local.get $wake-from-suspend-argp) (call $thread-index))
                (i32.store offset=4 (local.get $wake-from-suspend-argp) (local.get 0))
                ;; Spawn a new thread that will wake us up from our uncancellable suspend and schedule
                ;; it to resume after we suspend.
                (call $thread-resume-later 
                    (call $thread-new-indirect (i32.const 0) (local.get $wake-from-suspend-argp)))

                ;; Request suspension. We will not be woken up by cancellation, because this is an uncancellable
                ;; suspend. We will be woken up by the other thread we spawned above, which will be resumed after
                ;; the supertask cancels our subtask.
                (if (i32.ne (call $thread-suspend) (i32.const 0)) (then unreachable))
                ;; Request suspension again. This time we should see the cancellation immediately.
                (if (i32.ne (call $thread-suspend-cancellable) (i32.const 1)) (then unreachable))
                (call $task-cancel)
            )
        ) 

        ;; Instantiate the libc module to get the table
        (core instance $libc (instantiate $libc))
        ;; Get access to `thread.new_indirect` that uses the table from libc
        (core type $start-func-ty (func (param i32)))
        (alias core export $libc "__indirect_function_table" (core table $indirect-function-table))

        (core func $task-cancel (canon task.cancel))
        (core func $thread-new-indirect 
            (canon thread.new_indirect $start-func-ty (table $indirect-function-table)))
        (core func $thread-yield (canon thread.yield))
        (core func $thread-yield-cancellable (canon thread.yield cancellable))
        (core func $thread-index (canon thread.index))
        (core func $thread-yield-to (canon thread.yield-to))
        (core func $thread-resume-later (canon thread.resume-later))
        (core func $thread-switch-to (canon thread.switch-to))
        (core func $thread-suspend (canon thread.suspend))
        (core func $thread-suspend-cancellable (canon thread.suspend cancellable))
        (core func $future.read (canon future.read $FT async (memory $memory "mem")))
        (core func $waitable-set.new (canon waitable-set.new))
        (core func $waitable.join (canon waitable.join))
        (core func $waitable-set.wait (canon waitable-set.wait (memory $memory "mem")))

        ;; Instantiate the main module
        (core instance $cm (
            instantiate $CM
                (with "" (instance
                    (export "mem" (memory $memory "mem"))
                    (export "task.cancel" (func $task-cancel))
                    (export "thread.new_indirect" (func $thread-new-indirect))
                    (export "thread.index" (func $thread-index))
                    (export "thread.yield-to" (func $thread-yield-to))
                    (export "thread.yield" (func $thread-yield))
                    (export "thread.yield-cancellable" (func $thread-yield-cancellable))
                    (export "thread.switch-to" (func $thread-switch-to))
                    (export "thread.suspend" (func $thread-suspend))
                    (export "thread.suspend-cancellable" (func $thread-suspend-cancellable))
                    (export "thread.resume-later" (func $thread-resume-later))
                    (export "future.read" (func $future.read))
                    (export "waitable.join" (func $waitable.join))
                    (export "waitable-set.wait" (func $waitable-set.wait))
                    (export "waitable-set.new" (func $waitable-set.new))))
                (with "libc" (instance $libc))))

        (func (export "run-yield") (result u32) (canon lift (core func $cm "run-yield") async))
        (func (export "run-suspend") (param "fut" $FT) (result u32) (canon lift (core func $cm "run-suspend") async))
    )
    (component $D 
        (type $FT (future))
        (import "run-yield" (func $run-yield (result u32)))
        (import "run-suspend" (func $run-suspend (param "fut" $FT) (result u32)))

        (core module $Memory (memory (export "mem") 1))
        (core instance $memory (instantiate $Memory))
        (core module $DM
            (import "" "mem" (memory 1))
            (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
            (import "" "run-yield" (func $run-yield (param i32) (result i32)))
            (import "" "run-suspend" (func $run-suspend (param i32 i32) (result i32)))
            (import "" "waitable.join" (func $waitable.join (param i32 i32)))
            (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
            (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
            (import "" "future.new" (func $future.new (result i64)))
            (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
            (import "" "thread.yield" (func $thread-yield (result i32)))
            (func $test-yield (result i32)
                (local $ret i32) (local $subtask i32)
                (local $ws i32) (local $event_code i32)
                (local $run-yield-retp i32) (local $wait-retp i32)

                ;; Set up return value storage for run-yield and waitable-set.wait
                (local.set $run-yield-retp (i32.const 4))
                (local.set $wait-retp (i32.const 8))
                (i32.store (local.get $run-yield-retp) (i32.const 0xbad0bad0))
                (i32.store (local.get $wait-retp) (i32.const 0xbad0bad0))

                ;; Calling run-yield will start the thread, which will yield
                (local.set $ret (call $run-yield (local.get $run-yield-retp)))
                ;; Ensure that the thread started
                (if (i32.ne (i32.and (local.get $ret) (i32.const 0xF)) (i32.const 1 (; STARTED ;)))
                 (then unreachable))
                ;; Extract the subtask index
                (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))
                ;; Cancel the subtask, which should block, because the initial yield is uncancellable
                (local.set $ret (call $subtask.cancel (local.get $subtask)))
                ;; Ensure the cancellation blocked
                (if (i32.ne (local.get $ret) (i32.const -1 (; BLOCKED ;)))
                 (then unreachable))

                ;; Wait on the subtask, which will cause it to resume, see the cancellation, and exit
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

            (func $test-suspend (result i32)
                (local $ret i32) (local $subtask i32)
                (local $ws i32) (local $event_code i32)
                (local $run-suspend-retp i32) (local $wait-retp i32)
                (local $ret64 i64) (local $futr i32) (local $futw i32)

                ;; Set up return value storage for run-suspend and waitable-set.wait
                (local.set $run-suspend-retp (i32.const 4))
                (local.set $wait-retp (i32.const 8))
                (i32.store (local.get $run-suspend-retp) (i32.const 0xbad0bad0))
                (i32.store (local.get $wait-retp) (i32.const 0xbad0bad0))

                ;; Create a future that the run-suspend thread will wait on
                (local.set $ret64 (call $future.new))
                (local.set $futr (i32.wrap_i64 (local.get $ret64)))
                (local.set $futw (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))

                ;; Calling run-suspend will start the thread, which will suspend
                (local.set $ret (call $run-suspend (local.get $futr) (local.get $run-suspend-retp)))
                ;; Ensure that the thread started
                (if (i32.ne (i32.and (local.get $ret) (i32.const 0xF)) (i32.const 1 (; STARTED ;)))
                 (then unreachable))
                ;; Extract the subtask index
                (local.set $subtask (i32.shr_u (local.get $ret) (i32.const 4)))
                ;; Cancel the subtask, which should block, because the initial suspend is uncancellable
                (local.set $ret (call $subtask.cancel (local.get $subtask)))
                ;; Ensure the cancellation blocked
                (if (i32.ne (local.get $ret) (i32.const -1 (; BLOCKED ;)))
                 (then unreachable))

                ;; Yield, ensuring the subtask's spawned thread gets to run
                (if (i32.ne (call $thread-yield) (i32.const 0)) (then unreachable))

                ;; Write to the future, which the subtask's spawned thread is waiting on
                (local.set $ret (call $future.write (local.get $futw) (i32.const 0xdeadbeef)))
                ;; The write should succeed
                (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
                    (then unreachable))
                ;; Wait on the subtask, which will cause its spawned thread to wake up the main thread,
                ;; which will then enact a cancellable suspend, see the cancellation, and exit
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
                (if (i32.ne (call $test-yield) (i32.const 42))
                    (then unreachable))
                (if (i32.ne (call $test-suspend) (i32.const 42))
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
        (core func $future.write (canon future.write $FT async (memory $memory "mem")))
        (core func $thread.yield (canon thread.yield))
        (canon lower (func $run-yield) async (memory $memory "mem") (core func $run-yield'))
        (canon lower (func $run-suspend) async (memory $memory "mem") (core func $run-suspend'))
        (core instance $dm (instantiate $DM (with "" (instance
            (export "mem" (memory $memory "mem"))
            (export "run-yield" (func $run-yield'))
            (export "run-suspend" (func $run-suspend'))
            (export "waitable.join" (func $waitable.join))
            (export "waitable-set.new" (func $waitable-set.new))
            (export "waitable-set.wait" (func $waitable-set.wait))
            (export "subtask.cancel" (func $subtask.cancel))
            (export "future.new" (func $future.new))
            (export "future.write" (func $future.write))
            (export "thread.yield" (func $thread.yield))
        ))))
        (func (export "run") (result u32) (canon lift (core func $dm "run")))
    )

    (instance $c (instantiate $C))
    (instance $d (instantiate $D
        (with "run-yield" (func $c "run-yield"))
        (with "run-suspend" (func $c "run-suspend"))
    ))
  (func (export "run") (alias export $d "run"))
)

(assert_return (invoke "run") (u32.const 42))