(module
  ;; As we have discussed, it makes sense to make the shared memory an import
  ;; so that all
  (import "" "memory" (memory $shmem 1 1 shared))
  (import "wasi_snapshot_preview1" "fd_write"
    (func $__wasi_fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "proc_exit"
    (func $__wasi_proc_exit (param i32)))
  (import "wasi" "thread-spawn"
    (func $__wasi_thread_spawn (param i32) (result i32)))

  (func (export "_start")
    (local $i i32)

    ;; Print "Called _start".
    (call $print (i32.const 32) (i32.const 14))

    ;; Print "Running wasi_thread_start" in several threads.
    (drop (call $__wasi_thread_spawn (i32.const 0)))
    (drop (call $__wasi_thread_spawn (i32.const 0)))
    (drop (call $__wasi_thread_spawn (i32.const 0)))

    ;; Wait for all the threads to notify us that they are done.
    (local.set $i (i32.const 0))
    (loop $again
      ;; Wait for the i32 at address 128 to be incremented by each thread. We
      ;; maintain a local $i with the atomically loaded value as the expected
      ;; wait value and to check if all three threads are complete. This wait is
      ;; for 1ms or until notified, whichever is first.
      (drop (memory.atomic.wait32 (i32.const 128) (local.get $i) (i64.const 1000000)))
      (local.set $i (i32.atomic.load (i32.const 128)))
      (br_if $again (i32.lt_s (local.get $i) (i32.const 3)))
    )

    ;; Print "Done".
    (call $print (i32.const 64) (i32.const 5))
  )

  ;; A threads-enabled module must export this spec-designated entry point.
  (func (export "wasi_thread_start") (param $tid i32) (param $start_arg i32)
    (call $print (i32.const 96) (i32.const 26))
    ;; After printing, we atomically increment the value at address 128 and then
    ;; wake up the main thread's join loop.
    (drop (i32.atomic.rmw.add (i32.const 128) (i32.const 1)))
    (drop (memory.atomic.notify (i32.const 128) (i32.const 1)))
  )

  ;; A helper function for printing ptr-len strings.
  (func $print (param $ptr i32) (param $len i32)
    (i32.store (i32.const 8) (local.get $len))
    (i32.store (i32.const 4) (local.get $ptr))
        (drop (call $__wasi_fd_write
          (i32.const 1)
          (i32.const 4)
          (i32.const 1)
          (i32.const 0)))
  )

  ;; We still need to export the shared memory for Wiggle's sake.
  (export "memory" (memory $shmem))

  (data (i32.const 32) "Called _start\0a")
  (data (i32.const 64) "Done\0a")
  (data (i32.const 96) "Running wasi_thread_start\0a")
)
