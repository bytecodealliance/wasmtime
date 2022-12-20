(module
  ;; As we have discussed, it makes sense to make the shared memory an import
  ;; so that all
  (import "" "memory" (memory $shmem 1 1 shared))
  (import "wasi_snapshot_preview1" "fd_write"
    (func $__wasi_fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi" "thread_spawn"
    (func $__wasi_thread_spawn (param i32) (result i32)))

  (func (export "_start")
    (local $i i32)

    ;; Print "Hello _start".
    (call $print (i32.const 32) (i32.const 13))

    ;; Print "Hello wasi_thread_start" in several threads.
    (drop (call $__wasi_thread_spawn (i32.const 0)))
    (drop (call $__wasi_thread_spawn (i32.const 0)))
    (drop (call $__wasi_thread_spawn (i32.const 0)))

    ;; Wasmtime has no `wait/notify` yet, so we just spin to allow the threads
    ;; to do their work.
    (local.set $i (i32.const 2000000))
    (loop $again
      (local.set $i (i32.sub (local.get $i) (i32.const 1)))
      (br_if $again (i32.gt_s (local.get $i) (i32.const 0)))
    )

    ;; Print "Hello done".
    (call $print (i32.const 64) (i32.const 11))
  )

  ;; A threads-enabled module must export this spec-designated entry point.
  (func (export "wasi_thread_start") (param $tid i32) (param $start_arg i32)
    (call $print (i32.const 96) (i32.const 24))
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

  (data (i32.const 32) "Hello _start\0a")
  (data (i32.const 64) "Hello done\0a")
  (data (i32.const 96) "Hello wasi_thread_start\0a")
)
