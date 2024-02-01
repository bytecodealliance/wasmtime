;; Read the directory entries of the preopened directory.
(module
    (import "wasi_snapshot_preview1" "fd_readdir"
        (func $__wasi_fd_readdir (param i32 i32 i32 i64 i32) (result i32)))
    (func (export "run") (param $iters i64) (result i64)
        (local $i i64)
        (local.set $i (i64.const 0))

        (if (i32.ne (i32.load (i32.const 0)) (i32.const 0))
            (then unreachable))

        (loop $cont
            ;; Read the file into the sole iovec buffer.
            (call $__wasi_fd_readdir
                ;; The fd of the preopened directory; the first three are the
                ;; `std*` ones.
                (i32.const 3)
                ;; The buffer address at which to store the entries and the
                ;; length of the buffer.
                (i32.const 16)
                (i32.const 4096)
                ;; The location at which to start reading entries in the
                ;; directory; here we start at the first entry.
                (i64.const 0)
                ;; The address at which to store the number of bytes read.
                (i32.const 8))
            (drop)

            ;; Check that we indeed read at least 380 bytes of directory
            ;; entries.
            (if (i32.lt_u (i32.load (i32.const 8)) (i32.const 300))
               (then unreachable))

            ;; Continue looping until $i reaches $iters.
            (local.set $i (i64.add (local.get $i) (i64.const 1)))
            (br_if $cont (i64.lt_u (local.get $i) (local.get $iters)))
        )
        (local.get $i)
    )
    (memory (export "memory") 1)
)
