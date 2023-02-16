(module
    (import "wasi_snapshot_preview1" "args_get"
        (func $__wasi_args_get (param i32 i32) (result i32)))
    (import "wasi_snapshot_preview1" "args_sizes_get"
        (func $__wasi_args_sizes_get (param i32 i32) (result i32)))
    (func (export "run") (param $iters i64) (result i64)
        (local $i i64)
        (local.set $i (i64.const 0))
        (loop $cont
            ;; Read the current argument list by:
            ;;  1) retrieving the argument sizes and then
            ;;  2) retrieving the argument data itself.

            ;; Retrieve the sizes of the arguments with parameters:
            ;; - the address at which to write the number of arguments
            ;; - the address at which to write the size of the argument buffer
            ;; Returns an error code.
            (call $__wasi_args_sizes_get (i32.const 0) (i32.const 4))
            (drop)

            ;; Read the arguments with parameters:
            ;; - the address at which to write the array of argument pointers
            ;;   (i.e., one pointer per argument); here we overwrite the size
            ;;   written at address 0
            ;; - the address at which to write the buffer of argument strings
            ;;   (pointed to by the items written to the first address); we
            ;;   calculate where to start the buffer based on the size of the
            ;;   pointer list (i.e., number of arguments * 4 bytes per pointer)
            ;; Returns an error code.
            (call $__wasi_args_get
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
