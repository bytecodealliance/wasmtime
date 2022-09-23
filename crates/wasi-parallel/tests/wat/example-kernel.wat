(module
    (memory (import "" "memory") 1 1 shared)
    (func $kernel (export "kernel") (param $iteration_id i32) (param $num_iterations i32) (param $block_size i32)
        (i32.store
            ;; Address of shared memory to store at--no buffer used.
            (i32.const 0)
            ;; Increment the iteration ID to avoid 0 and store this.
            (i32.add (local.get $iteration_id) (i32.const 1))
        ))
)
