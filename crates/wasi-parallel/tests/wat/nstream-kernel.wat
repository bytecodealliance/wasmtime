;; This hand-coded implementation of the nstream splits the work
;; dynamically--i.e., in the kernel itself. See, e.g.,
;; https://github.com/ParRes/Kernels/blob/default/Cxx11/nstream-tbb.cc#L132 for
;; a higher-level implementation.

(module
    (memory (import "" "memory") 0x800 0x800 shared)
    (func $kernel (export "kernel") (param $iteration_id i32) (param $num_iterations i32) (param $block_size i32) (param $A i32) (param $A_len i32) (param $B i32) (param $B_len i32) (param $C i32) (param $C_len i32)
        (local $A_i i32)
        (local $B_i i32)
        (local $C_i i32)
        (local $i i32)
        (local $end i32)

        ;; The division of the buffers between iterations happens here: the
        ;; block size defines how many floating-point numbers each iteration
        ;; will touch:
        ;; i = iteration_id * block_size * 4;
        ;; end = i + block_size * 4;
        (local.set $i (i32.mul (i32.mul (local.get $iteration_id) (local.get $block_size)) (i32.const 4)))
        (local.set $end (i32.add (local.get $i) (i32.mul (local.get $block_size) (i32.const 4))))

        (block
            (loop
                (local.set $A_i (i32.add (local.get $A) (local.get $i)))
                (local.set $B_i (i32.add (local.get $B) (local.get $i)))
                (local.set $C_i (i32.add (local.get $C) (local.get $i)))

                ;; Offset to store: A[i] = ...
                (local.get $A_i)
                ;; Value to store:   ... = A[i] + B[i] + 3.0 * C[i];
                (f32.add
                    (f32.load (local.get $A_i))
                    (f32.add
                        (f32.load (local.get $B_i))
                        (f32.mul
                            (f32.const 3.0)
                            (f32.load (local.get $C_i)))))
                (f32.store)

                ;; Loop control--exit once we have looped through this
                ;; iteration's portion of the buffer.
                (local.set $i (i32.add (local.get $i) (i32.const 4)))
                (i32.ge_s (local.get $i) (local.get $end))
                (br_if 1)
                (br 0))))
)
