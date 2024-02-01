;; Repeatedly read the contents of `test.bin`.
(module
    (import "wasi_snapshot_preview1" "path_open"
        (func $__wasi_path_open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
    (import "wasi_snapshot_preview1" "fd_read"
        (func $__wasi_fd_read (param i32 i32 i32 i32) (result i32)))
    (import "wasi_snapshot_preview1" "fd_close"
        (func $__wasi_fd_close (param i32) (result i32)))
    (func (export "run") (param $iters i64) (result i64)
        (local $i i64)
        (local.set $i (i64.const 0))

        ;; Set up the iovec list; the memory usage for this module should be:
        ;; - offset 0 => file name
        ;; - offset 16 => the opened file descriptor
        ;; - offset 24 => the number of read bytes
        ;; - offset 32 => the iovec list
        ;; - offset 48 => the first (and only) iovec buffer
        (i32.store (i32.const 32) (i32.const 48))
        (i32.store (i32.const 36) (i32.const 4096))

        (loop $cont
            ;; Open the file `test.bin` under the same directory as this WAT
            ;; file; this assumes some prior set up of the preopens in
            ;; `wasi.rs`. See https://github.com/WebAssembly/WASI/blob/d8da230b/phases/snapshot/witx/wasi_snapshot_preview1.witx#L346.
            (call $__wasi_path_open
                ;; The fd of the preopen under which to search for the file;
                ;; the first three are the `std*` ones.
                (i32.const 3)
                ;; The lookup flags (i.e., whether to follow symlinks).
                (i32.const 0)
                ;; The path to the file under the initial fd.
                (i32.const 0)
                (i32.const 8)
                ;; The open flags; in this case we will only attempt to read but
                ;; this may attempt to create the file if it does not exist, see
                ;; https://github.com/WebAssembly/WASI/blob/d8da230b/phases/snapshot/witxtypenames.witx#L444).
                (i32.const 0)
                ;; The base rights and the inheriting rights: here we only set
                ;; the bits for the FD_READ and FD_READDIR capabilities.
                (i64.const 0x2002)
                (i64.const 0x2002)
                ;; The file descriptor flags (e.g., whether to append, sync,
                ;; etc.); see https://github.com/WebAssembly/WASI/blob/d8da230b/phases/snapshot/witx/typenames.witx#L385
                (i32.const 0)
                ;; The address at which to store the opened fd (if the call
                ;; succeeds)
                (i32.const 16))
            (if (then unreachable))

            ;; Read the file into the sole iovec buffer.
            (call $__wasi_fd_read
                ;; The now-open fd stored at offset 16.
                (i32.load (i32.const 16))
                ;; The address and size of the list of iovecs; here we only use
                ;; a list of a single iovec set up outside the loop.
                (i32.const 32)
                (i32.const 1)
                ;; The address at which to store the number of bytes read.
                (i32.const 24))
            (if (then unreachable))
            ;; Check that we indeed read 4096 bytes.
            (if (i32.ne (i32.load (i32.const 24)) (i32.const 4096))
                (then unreachable))

            ;; Close the open file handle we stored at offset 16.
            (call $__wasi_fd_close (i32.load (i32.const 16)))
            (if (then unreachable))

            ;; Continue looping until $i reaches $iters.
            (local.set $i (i64.add (local.get $i) (i64.const 1)))
            (br_if $cont (i64.lt_u (local.get $i) (local.get $iters)))
        )
        (local.get $i)
    )
    (data (i32.const 0) "test.bin")
    (memory (export "memory") 1)
)
