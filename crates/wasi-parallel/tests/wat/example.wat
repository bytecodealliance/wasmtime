;; A minimal example of wasi-parallel execution--each thread of execution writes
;; to the same location in memory. This makes use of the implicit detail in the
;; current implementation that allows a CPU-run kernel to modify the module's
;; memory.
;;
;; Note how both the kernel code is accessible in linear memory. Without
;; read-only access support in WebAssembly, the kernel could be overwritten
;; (accidentally or maliciously), which is a risk in this "bag-of-bytes"
;; paradigm.

(module
    (import "wasi_ephemeral_parallel" "get_device" (func $get_device
        (param $hint i32)
        (param $out_device i32)
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

    ;; The kernel here is the binary-encoded version of `example-kernel.wat`,
    ;; using:
    ;;
    ;; $ wat2wasm tests/wat/example-kernel.wat --enable-threads --output=- | xxd -g 1 -p | sed -r 's/.{2}/\\&/g' | tr -d '\n'
    ;;
    ;; The length is calculated using `wc -c` and dividing by 3.
    (memory (export "memory") 1 1 shared)
    ;; Reserve 8 bytes for the return area, then emit the kernel:
    (data (i32.const 8) "\00\61\73\6d\01\00\00\00\01\07\01\60\03\7f\7f\7f\00\02\0d\01\00\06\6d\65\6d\6f\72\79\02\03\01\01\03\02\01\00\07\0a\01\06\6b\65\72\6e\65\6c\00\00\0a\0e\01\0c\00\41\00\20\00\41\01\6a\36\02\00\0b")

    (func (export "_start") (result i32)
        (local $return_area i32)
        (local $device i32)
        (local.set $return_area (i32.const 0))

        ;; Set up a CPU device.
        (drop (call $get_device (i32.const 0x01) (local.get $return_area)))
        (local.set $device (i32.load (local.get $return_area)))

        ;; Execute the kernel in parallel.
        (call $par_exec (local.get $device) (i32.const 8) (i32.const 64) (i32.const 12) (i32.const 4)
            ;; Empty buffers:
            (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0))

        ;; Check that the parallel execution returned 0 (success) and that the
        ;; memory was updated by an invocation of the kernel--if so, return 0.
        (i32.eq (i32.load (i32.const 0)) (i32.const 0))
        (i32.or))
)
