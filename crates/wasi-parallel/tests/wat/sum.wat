;; This hand-coded implementation of nstream splits the work--a 32MB buffer (4 *
;; LLC)--evenly among 4 cores and uses wasi-parallel to distribute the work.
;; See, e.g.,
;; https://github.com/ParRes/Kernels/blob/default/Cxx11/nstream-tbb.cc#L132 for
;; a higher-level implementation.

(module
    (import "wasi_ephemeral_parallel" "get_device" (func $get_device
        (param $hint i32)
        (param $out_device i32)
        (result i32)))
    (import "wasi_ephemeral_parallel" "create_buffer" (func $create_buffer
        (param $device i32)
        (param $size i32)
        (param $access i32)
        (param $out_buffer i32)
        (result i32)))
    (import "wasi_ephemeral_parallel" "write_buffer" (func $write_buffer
        (param $data_offset i32)
        (param $data_len i32)
        (param $buffer i32)
        (result i32)))
    (import "wasi_ephemeral_parallel" "read_buffer" (func $read_buffer
        (param $buffer i32)
        (param $data_offset i32)
        (param $data_len i32)
        (result i32)))
    (import "wasi_ephemeral_parallel" "parallel_exec" (func $par_exec
        (param $device i32)
        (param $kernel_start i32)
        (param $kernel_len i32)
        (param $num_iterations i32)
        (param $block_size i32)
        (param $in_buffers_start i32)
        (param $in_buffers_len i32)
        (param $out_buffers_start i32)
        (param $out_buffers_len i32)
        (result i32)))

    ;; The kernel here is the binary-encoded version of `nstream-kernel.wat`,
    ;; using:
    ;;
    ;; $ wat2wasm tests/wat/sum-kernel.wat --enable-threads --output=- | xxd -g 1 -p | sed -r 's/.{2}/\\&/g' | tr -d '\n'
    ;;
    ;; The length is calculated using `wc -c` and dividing by 3.
    (memory (export "memory") 1 1 shared)
    ;; Reserve 8 bytes for the return area and buffer lists, then emit the
    ;; kernel:
    (data (i32.const 8) "\00\61\73\6d\01\00\00\00\01\09\01\60\05\7f\7f\7f\7f\7f\00\02\0d\01\00\06\6d\65\6d\6f\72\79\02\03\01\01\03\02\01\00\07\0a\01\06\6b\65\72\6e\65\6c\00\00\0a\2d\01\2b\01\02\7e\42\00\21\05\20\02\ad\21\06\03\40\20\05\42\01\7c\21\05\20\05\20\06\54\0d\00\0b\20\03\20\00\41\08\6c\6a\20\05\37\03\00\0b")

    ;; Global values overwritten by `setup`.
    (global $num_threads (mut i32) (i32.const 4))
    (global $block_size (mut i32) (i32.const 0x2000000))
    (global $buffer_size (mut i32) (i32.const 0x2000000))
    (global $device (mut i32) (i32.const -1))
    (global $A (mut i32) (i32.const -1))
    (global $memA i32 (i32.const 0x1000))

    (func (export "setup") (param $num_threads i32) (param $block_size i32) (param $device_kind i32)
        (local $return_area i32)
        ;; Assign the return area pointer.
        (local.set $return_area (i32.const 0x00))

        ;; Save some setup parameters for later.
        (global.set $num_threads (local.get $num_threads))
        (global.set $buffer_size (i32.mul (local.get $num_threads) (i32.const 8)))
        (global.set $block_size (local.get $block_size))

        ;; Set up the device.
        (drop (call $get_device (local.get $device_kind) (local.get $return_area)))
        (global.set $device (i32.load (local.get $return_area)))

        ;; Create a buffer to store the intermediate results. Note that `0x01 = read-write`.
        (drop (call $create_buffer (global.get $device) (global.get $buffer_size) (i32.const 0x01) (local.get $return_area)))
        (global.set $A (i32.load (local.get $return_area)))

        ;; Assign the buffer its (empty) contents.
        (drop (call $write_buffer (global.get $memA) (global.get $buffer_size) (global.get $A)))
    )

    (func (export "execute")
        (local $sum i64)
        (local $i i32)

        ;; Set up the list of buffers.
        (i32.store (i32.const 0) (global.get $A))

        ;; Execute the kernel in parallel.
        (call $par_exec (global.get $device)
            ;; Kernel bytes.
            (i32.const 8) (i32.const 97)
            ;; Number of iterations and block size
            (global.get $num_threads) (global.get $block_size)
            ;; Input buffers.
            (i32.const 0) (i32.const 1)
            ;; Output buffers.
            (i32.const 0) (i32.const 0))
        (drop)

        ;; Read the buffer contents.
        (drop (call $read_buffer (global.get $A) (global.get $memA) (global.get $buffer_size)))

        ;; Calculate the sum of each iteration's work and store it at address 0.
        (local.set $i (i32.const 0))
        (loop $cont
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (local.set $sum (i64.add
                (local.get $sum)
                (i64.load (i32.add
                    (global.get $memA)
                    (i32.mul (local.get $i) (i32.const 8))
                ))
            ))
            (br_if $cont (i32.lt_u (local.get $i) (global.get $num_threads)))
        )
        (i64.store (i32.const 0) (local.get $sum))
    )

    (func (export "finish") (result i32)
        ;; Assert that the aggregate sum is what is expected.
        (i64.eq
            (i64.load (i32.const 0))
            (i64.mul
                (i64.extend_i32_u (global.get $num_threads))
                (i64.extend_i32_u (global.get $block_size))
            )
        )
    )
)
