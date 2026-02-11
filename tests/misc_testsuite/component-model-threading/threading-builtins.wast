;;! component_model_async = true
;;! component_model_threading = true

;; Tests for basic functioning of all threading builtins with the implicit thread + one explicit thread
;; Switches between threads using all of the different threading intrinsics.

(component
    ;; Defines the table for the thread start function
    (core module $libc
        (table (export "__indirect_function_table") 1 funcref))
    ;; Defines the thread start function and a function that calls thread.new-indirect
    (core module $m
        ;; Import the threading builtins and the table from libc
        (import "" "thread.new-indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
        (import "" "thread.suspend" (func $thread-suspend (result i32)))
        (import "" "thread.yield-to-suspended" (func $thread-yield-to-suspended (param i32) (result i32)))
        (import "" "thread.suspend-to-suspended" (func $thread-suspend-to-suspended (param i32) (result i32)))
        (import "" "thread.yield" (func $thread-yield (result i32)))
        (import "" "thread.index" (func $thread-index (result i32)))
        (import "" "thread.unsuspend" (func $thread-unsuspend (param i32)))
        (import "libc" "__indirect_function_table" (table $indirect-function-table 1 funcref))

        ;; A global that we will set from the spawned thread
        (global $g (mut i32) (i32.const 0))
        (global $main-thread-index (mut i32) (i32.const 0))

        ;; The thread entry point, which sets the global to incrementing values starting from the context value
        (func $thread-start (param i32)
            ;; Set the global to the context value
            (global.set $g (local.get 0))
            ;; The main thread switched to us, so is no longer scheduled, so we explicitly schedule it
            (call $thread-unsuspend (global.get $main-thread-index))
            ;; Yield back to the main thread (since that is the only other one)
            (drop (call $thread-yield)
            ;; Increment the global
            (global.set $g (i32.add (global.get $g) (i32.const 1)))
            ;; The main thread will have explicitly requested suspension, so yield to it directly
            (drop (call $thread-yield-to-suspended (global.get $main-thread-index)))
            ;; Increment the global again
            (global.set $g (i32.add (global.get $g) (i32.const 1)))
            ;; Reschedule the main thread so that it runs after we exit
            (call $thread-unsuspend (global.get $main-thread-index))))
        (export "thread-start" (func $thread-start))

        ;; Initialize the function table with our thread-start function; this will be
        ;; used by thread.new-indirect
        (elem (table $indirect-function-table) (i32.const 0) func $thread-start)

        ;; The main entry point, which spawns a new thread to run `thread-start`, passing 42
        ;; as the context value, and then yields to it
        (func (export "run") (result i32)
            ;; Store the main thread's index for the spawned thread to yield to
            (global.set $main-thread-index (call $thread-index))
            ;; Create a new thread, which starts suspended, and switch to it
            (drop
                (call $thread-suspend-to-suspended
                    (call $thread-new-indirect (i32.const 0) (i32.const 42))))
            ;; After the thread yields back to us, check that the global was set to 42
            (if (i32.ne (global.get $g) (i32.const 42)) (then unreachable))
            ;; Suspend ourselves, which will cause the spawned thread to run
            (drop (call $thread-suspend))
            ;; The spawned thread will resume us after incrementing the global, so check that it is now 43
            (if (i32.ne (global.get $g) (i32.const 43)) (then unreachable))
            ;; Suspend again, which will cause the spawned thread to run again
            (drop (call $thread-suspend))
            ;; The spawned thread will reschedule us before it exits, so when we resume here the global should be 44
            (if (i32.ne (global.get $g) (i32.const 44)) (then unreachable))
            ;; Return success
            (i32.const 42)))

    ;; Instantiate the libc module to get the table
    (core instance $libc (instantiate $libc))
    ;; Get access to `thread.new-indirect` that uses the table from libc
    (core type $start-func-ty (func (param i32)))
    (alias core export $libc "__indirect_function_table" (core table $indirect-function-table))

    (core func $thread-new-indirect
        (canon thread.new-indirect $start-func-ty (table $indirect-function-table)))
    (core func $thread-yield (canon thread.yield))
    (core func $thread-index (canon thread.index))
    (core func $thread-yield-to-suspended (canon thread.yield-to-suspended))
    (core func $thread-unsuspend (canon thread.unsuspend))
    (core func $thread-suspend-to-suspended (canon thread.suspend-to-suspended))
    (core func $thread-suspend (canon thread.suspend))

    ;; Instantiate the main module
    (core instance $i (
        instantiate $m
            (with "" (instance
                (export "thread.new-indirect" (func $thread-new-indirect))
                (export "thread.index" (func $thread-index))
                (export "thread.yield-to-suspended" (func $thread-yield-to-suspended))
                (export "thread.yield" (func $thread-yield))
                (export "thread.suspend-to-suspended" (func $thread-suspend-to-suspended))
                (export "thread.suspend" (func $thread-suspend))
                (export "thread.unsuspend" (func $thread-unsuspend))))
            (with "libc" (instance $libc))))

    ;; Export the main entry point
    (func (export "run") async (result u32) (canon lift (core func $i "run"))))

(assert_return (invoke "run") (u32.const 42))

;; Test that `thread.index` is exempt from may-leave checks
(component
  (core func $thread.index (canon thread.index))

  (core module $DM
    (import "" "thread.index" (func $thread.index (result i32)))

    (func (export "run"))
    (func (export "post-return") call $thread.index drop)
  )
  (core instance $dm (instantiate $DM (with "" (instance
    (export "thread.index" (func $thread.index))
  ))))
  (func (export "run")
    (canon lift (core func $dm "run") (post-return (func $dm "post-return"))))
)

(assert_return (invoke "run"))
