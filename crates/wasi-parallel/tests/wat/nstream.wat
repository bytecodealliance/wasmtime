;; This hand-coded implementation of `nstream` uses wasi-parallel to split the
;; `nstream` work--some light arithmetic with heavy memory access--across the
;; parallelism available to `wasi-parallel`.
;;
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
    ;; $ wat2wasm tests/wat/nstream-kernel.wat --enable-threads --output=- | xxd -g 1 -p | sed -r 's/.{2}/\\&/g' | tr -d '\n'
    ;;
    ;; The length is calculated using `wc -c` and dividing by 3.
    (memory (export "memory") 0x800 0x800 shared)
    ;; Reserve 12 bytes for the return area and buffer lists, then emit the
    ;; kernel:
    (data (i32.const 12) "\00\61\73\6d\01\00\00\00\01\0d\01\60\09\7f\7f\7f\7f\7f\7f\7f\7f\7f\00\02\0f\01\00\06\6d\65\6d\6f\72\79\02\03\80\10\80\10\03\02\01\00\07\0a\01\06\6b\65\72\6e\65\6c\00\00\0a\61\01\5f\01\05\7f\20\00\20\02\6c\41\04\6c\21\0c\20\0c\20\02\41\04\6c\6a\21\0d\02\40\03\40\20\03\20\0c\6a\21\09\20\05\20\0c\6a\21\0a\20\07\20\0c\6a\21\0b\20\09\20\09\2a\02\00\20\0a\2a\02\00\43\00\00\40\40\20\0b\2a\02\00\94\92\92\38\02\00\20\0c\41\04\6a\21\0c\20\0c\20\0d\4e\0d\01\0c\00\0b\0b\0b")

    ;; Global values overwritten by `setup`.
    (global $num_threads (mut i32) (i32.const -1))
    (global $block_size (mut i32) (i32.const -1))
    (global $buffer_size (mut i32) (i32.const -1))
    (global $device (mut i32) (i32.const -1))
    (global $A (mut i32) (i32.const -1))
    (global $B (mut i32) (i32.const -1))
    (global $C (mut i32) (i32.const -1))


    (func (export "setup") (param $num_threads i32) (param $num_items i32) (param $device_kind i32)
        (local $return_area i32)
        (local $device i32)
        (local $len i32)
        (local $memA i32)
        (local $memB i32)
        (local $memC i32)
        (local $A i32)
        (local $B i32)
        (local $C i32)

        ;; Save some setup parameters for later.
        (global.set $num_threads (local.get $num_threads))
        (global.set $block_size (i32.add
            (i32.div_u (local.get $num_items) (local.get $num_threads))
            ;; Over-estimate the block size by 1 if there is a remainder to
            ;; account for.
            (if (result i32) (i32.rem_u (local.get $num_items) (local.get $num_threads))
                (then (i32.const 1))
                (else (i32.const 0))
            )
        ))
        (global.set $buffer_size (i32.mul
            (i32.mul (global.get $block_size) (i32.const 4))
            (global.get $num_threads)
        ))

        ;; Assign some pointers, skipping the first section of memory because
        ;; it contains the return area, the kernel bytes, etc.
        (local.set $return_area (i32.const 0x00))
        (local.set $memA (global.get $buffer_size))
        (local.set $memB (i32.mul (global.get $buffer_size) (i32.const 2)))
        (local.set $memC (i32.mul (global.get $buffer_size) (i32.const 3)))

        ;; Set up the device.
        (drop (call $get_device (local.get $device_kind) (local.get $return_area)))
        (global.set $device (i32.load (local.get $return_area)))

        ;; Create the buffers. Note that `0x00 = read` and `0x01 = read-write`.
        (drop (call $create_buffer (global.get $device) (global.get $buffer_size) (i32.const 0x01) (local.get $return_area)))
        (global.set $A (i32.load (local.get $return_area)))
        (drop (call $create_buffer (global.get $device) (global.get $buffer_size) (i32.const 0x00) (local.get $return_area)))
        (global.set $B (i32.load (local.get $return_area)))
        (drop (call $create_buffer (global.get $device) (global.get $buffer_size) (i32.const 0x00) (local.get $return_area)))
        (global.set $C (i32.load (local.get $return_area)))

        ;; Fill the buffers with the correct values.
        (call $initialize (local.get $memA) (global.get $buffer_size) (f32.const 0))
        (call $initialize (local.get $memB) (global.get $buffer_size) (f32.const 2))
        (call $initialize (local.get $memC) (global.get $buffer_size) (f32.const 2))

        ;; Assign the buffers their contents.
        (drop (call $write_buffer (local.get $memA) (global.get $buffer_size) (global.get $A)))
        (drop (call $write_buffer (local.get $memB) (global.get $buffer_size) (global.get $B)))
        (drop (call $write_buffer (local.get $memC) (global.get $buffer_size) (global.get $C))))

    (func (export "execute")
        ;; Set up the list of buffers.
        (i32.store (i32.const 0) (global.get $A))
        (i32.store (i32.const 4) (global.get $B))
        (i32.store (i32.const 8) (global.get $C))

        ;; Execute the kernel in parallel.
        (call $par_exec (global.get $device)
            ;; Kernel bytes.
            (i32.const 12) (i32.const 155)
            ;; Number of iterations and block size
            (global.get $num_threads) (global.get $block_size)
            ;; Input buffers.
            (i32.const 0) (i32.const 3)
            ;; Output buffers.
            (i32.const 0) (i32.const 0))

        drop)

    (func (export "finish") (result i32)
        (local $memA i32)
        (local.set $memA (global.get $buffer_size))
        ;; Assert that all values in A equal 8.0.
        (call $check (local.get $memA) (global.get $buffer_size) (f32.const 8.0))
    )

    ;; Helper function to (inefficiently) initialize a block of memory.
    (func $initialize (param $offset i32) (param $len i32) (param $value f32)
        (block
            (loop
                (local.set $len (i32.sub (local.get $len) (i32.const 4)))
                (f32.store (i32.add (local.get $offset) (local.get $len)) (local.get $value))
                (i32.le_s (local.get $len) (i32.const 0))
                (br_if 1)
                (br 0)
            )
        )
    )

    ;; Helper function to check that an entire memory region matches the value.
    (func $check (param $offset i32) (param $len i32) (param $value f32) (result i32)
        (loop $cont
            (local.set $len (i32.sub (local.get $len) (i32.const 4)))
            ;; If the loaded value does not match, early return with a `1`
            ;; code.
            (i32.const 1)
            (br_if 1 (f32.ne
                (local.get $value)
                (f32.load (i32.add (local.get $offset) (local.get $len)))))
            (drop)
            ;; Continue iterating until we reach the end, exiting with a `0`
            ;; success code.
            (br_if $cont (i32.gt_s (local.get $len) (i32.const 0)))
        )
        (i32.const 0)
    )
)
