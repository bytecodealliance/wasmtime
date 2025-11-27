;;! component_model_async = true
;;! component_model_threading = true

;; Spawns 5 threads, makes them write their indices into a buffer out-of-order, then yields to them in-order,
;; ensuring that the yield order is as-expected.

;; More concretely:
;; Thread Index | Assigned Number
;;       1      |        16
;;       2      |        12
;;       3      |        04
;;       4      |        00
;;       5      |        08

;; After all threads have spawned and written their indices to the byte position given by their assigned number,
;; the buffer state will be:
;; 4 3 5 2 1

;; The main thread will then yield to these threads in the order that they are stored in the buffer,
;; and they will write their assigned number into the buffer, after the indices.
;; After all threads have been yielded to, the buffer contents will be:
;; 4 3 5 2 1 0 4 8 12 16

;; The main thread then ensures that the assigned numbers have been written into the correct locations.

(component
    ;; Defines the table for the thread start function
    (core module $libc
        (table (export "__indirect_function_table") 1 funcref))
    ;; Defines the thread start function and a function that calls thread.new-indirect
    (core module $m
        ;; Import the threading builtins and the table from libc
        (import "" "thread.new-indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
        (import "" "thread.suspend" (func $thread-suspend (result i32)))
        (import "" "thread.yield-to" (func $thread-yield-to (param i32) (result i32)))
        (import "" "thread.switch-to" (func $thread-switch-to (param i32) (result i32)))
        (import "" "thread.yield" (func $thread-yield (result i32)))
        (import "" "thread.index" (func $thread-index (result i32)))
        (import "" "thread.resume-later" (func $thread-resume-later (param i32)))
        (import "libc" "__indirect_function_table" (table $indirect-function-table 1 funcref))

        ;; A memory block that threads will write their thread indexes and assigned values into
        (memory 1)

        ;; A global that points to the next memory index to write into
        ;; We initialize this to 20 (threads * 4 bytes of storage per thread)
        (global $g (mut i32) (i32.const 20))

        ;; The thread entry point, which writes the thread's index into memory at the assigned location,
        ;; suspends back to the main thread, then writes the assigned value into memory
        (func $thread-start (param i32)
            ;; Store the thread index into the assigned location
            (i32.store (local.get 0) (call $thread-index))
            (drop (call $thread-suspend))
            (i32.store (global.get $g) (local.get 0))
            (global.set $g
                (i32.add (global.get $g) (i32.const 4))))
        (export "thread-start" (func $thread-start))

        ;; Initialize the function table with our thread-start function; this will be
        ;; used by thread.new-indirect
        (elem (table $indirect-function-table) (i32.const 0) func $thread-start)

        (func $new-thread (param i32)
            (drop
                (call $thread-yield-to
                    (call $thread-new-indirect (i32.const 0) (local.get 0)))))

        ;; The main entry point
        (func (export "run") (result i32)
            ;; Spawn 5 new threads with assigned numbers
            (call $new-thread (i32.const 16))
            (call $new-thread (i32.const 12))
            (call $new-thread (i32.const 4))
            (call $new-thread (i32.const 0))
            (call $new-thread (i32.const 8))

            ;; Yield to all threads in ascending order of assigned number
            (drop (call $thread-yield-to (i32.load (i32.const 0))))
            (drop (call $thread-yield-to (i32.load (i32.const 4))))
            (drop (call $thread-yield-to (i32.load (i32.const 8))))
            (drop (call $thread-yield-to (i32.load (i32.const 12))))
            (drop (call $thread-yield-to (i32.load (i32.const 16))))

            ;; Ensure all assigned numbers have been written to the buffer in order
            (if (i32.ne (i32.load (i32.const 20)) (i32.const 0)) (then unreachable))
            (if (i32.ne (i32.load (i32.const 24)) (i32.const 4)) (then unreachable))
            (if (i32.ne (i32.load (i32.const 28)) (i32.const 8)) (then unreachable))
            (if (i32.ne (i32.load (i32.const 32)) (i32.const 12)) (then unreachable))
            (if (i32.ne (i32.load (i32.const 36)) (i32.const 16)) (then unreachable))

            ;; Sentinel value
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
    (core func $thread-yield-to (canon thread.yield-to))
    (core func $thread-resume-later (canon thread.resume-later))
    (core func $thread-switch-to (canon thread.switch-to))
    (core func $thread-suspend (canon thread.suspend))

    ;; Instantiate the main module
    (core instance $i (
        instantiate $m
            (with "" (instance
                (export "thread.new-indirect" (func $thread-new-indirect))
                (export "thread.index" (func $thread-index))
                (export "thread.yield-to" (func $thread-yield-to))
                (export "thread.yield" (func $thread-yield))
                (export "thread.switch-to" (func $thread-switch-to))
                (export "thread.suspend" (func $thread-suspend))
                (export "thread.resume-later" (func $thread-resume-later))))
            (with "libc" (instance $libc))))

    ;; Export the main entry point
    (func (export "run") async (result u32) (canon lift (core func $i "run"))))

(assert_return (invoke "run") (u32.const 42))
