(module
    (import "wasi_snapshot_preview1" "clock_time_get"
        (func $__wasi_clock_time_get (param i32 i64 i32) (result i32)))
    (func (export "run") (param $iters i64) (result i64)
        (local $i i64)
        (local.set $i (i64.const 0))
        (loop $cont
            ;; Retrieve the current time with the following parameters:
            ;; - $clockid: here we use the enum value for $realtime
            ;; - $precision: the maximum lag, which we set to 0 here
            ;; - the address at which to write the u64 $timestamp
            ;; Returns an error code.
            (call $__wasi_clock_time_get (i32.const 1) (i64.const 0) (i32.const 0))
            (drop)
            ;; Continue looping until $i reaches $iters.
            (local.set $i (i64.add (local.get $i) (i64.const 1)))
            (br_if $cont (i64.lt_u (local.get $i) (local.get $iters)))
        )
        (local.get $i)
    )
    (memory (export "memory") 1)
)
