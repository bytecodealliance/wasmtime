(module
    (import "wasi_snapshot_preview1" "environ_get"
        (func $__wasi_environ_get (param i32 i32) (result i32)))
    (import "wasi_snapshot_preview1" "environ_sizes_get"
        (func $__wasi_environ_sizes_get (param i32 i32) (result i32)))
    (func (export "run") (param $iters i64) (result i64)
        (local $i i64)
        (local.set $i (i64.const 0))
        (loop $cont
            ;; Read the current environment key-value pairs by:
            ;;  1) retrieving the environment sizes and then
            ;;  2) retrieving the environment data itself.

            ;; Retrieve the sizes of the environment with parameters:
            ;; - the address at which to write the number of environment
            ;;   variables
            ;; - the address at which to write the size of the environment
            ;;   buffer
            ;; Returns an error code.
            (call $__wasi_environ_sizes_get (i32.const 0) (i32.const 4))
            (drop)

            ;; Read the environment with parameters:
            ;; - the address at which to write the array of environment pointers
            ;;   (i.e., one pointer per key-value pair); here we overwrite
            ;;   the size written at address 0
            ;; - the address at which to write the buffer of key-value pairs
            ;;   (pointed to by the items written to the first address); we
            ;;   calculate where to start the buffer based on the size of the
            ;;   pointer list (i.e., number of key-value pairs * 4 bytes per
            ;;   pointer)
            ;; Returns an error code.
            (call $__wasi_environ_get
                (i32.const 0)
                (i32.mul (i32.load (i32.const 0)) (i32.const 4)))
            (drop)

            ;; Continue looping until $i reaches $iters.
            (local.set $i (i64.add (local.get $i) (i64.const 1)))
            (br_if $cont (i64.lt_u (local.get $i) (local.get $iters)))
        )
        (local.get $i)
    )
    (memory (export "memory") 1)
)
