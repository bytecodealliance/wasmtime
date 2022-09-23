;; This trivial kernel increments a local until it matches the block size,
;; then it stores this value in a buffer to be aggregated later.

(module
    (memory (import "" "memory") 1 1 shared)
    (func $kernel (export "kernel") (param $iteration_id i32) (param $num_iterations i32) (param $block_size i32) (param $A i32) (param $A_len i32)
        (local $i i64)
        (local $end i64)
        (local.set $i (i64.const 0))
        (local.set $end (i64.extend_i32_u (local.get $block_size)))
        (loop $cont
            (local.set $i (i64.add (local.get $i) (i64.const 1)))
            (br_if $cont (i64.lt_u (local.get $i) (local.get $end)))
        )
        (i64.store
            ;; Address to store at.
            (i32.add (local.get $A) (i32.mul (local.get $iteration_id) (i32.const 8)))
            ;; The summed value.
            (local.get $i)
        )
    )
)
