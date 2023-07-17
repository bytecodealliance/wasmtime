(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))

  (import "wasi_snapshot_preview1" "args_get"
    (func $args_get (param i32 i32) (result i32)))

  (memory (export "memory") 1)

  (func (export "_start")
    (local $argptrs i32)
    (local $argmem i32)
    (local $arg i32)

    (local.set $argptrs (i32.mul (memory.grow (i32.const 1)) (i32.const 65536)))
    (local.set $argmem (i32.mul (memory.grow (i32.const 1)) (i32.const 65536)))

    (if (i32.ne
          (call $args_get (local.get $argptrs) (local.get $argmem))
          (i32.const 0))
        (unreachable))

    (loop
      (local.set $arg (i32.load (local.get $argptrs)))
      (local.set $argptrs (i32.add (local.get $argptrs) (i32.const 4)))
      (if (i32.eq (local.get $arg) (i32.const 0)) (return))

      (call $write_all (local.get $arg) (call $strlen (local.get $arg)))
      (call $write_all (i32.const 10) (i32.const 1))
      br 0
    )
  )

  (func $write_all (param $ptr i32) (param $len i32)
    (local $rc i32)
    (local $iov i32)
    (local $written i32)

    (local.set $written (i32.const 80))
    (local.set $iov (i32.const 100))

    (loop
      (local.get $len)
      if
        (i32.store (local.get $iov) (local.get $ptr))
        (i32.store offset=4 (local.get $iov) (local.get $len))
        (local.set $rc
          (call $fd_write
            (i32.const 1)
            (local.get $iov)
            (i32.const 1)
            (local.get $written)))
        (if (i32.ne (local.get $rc) (i32.const 0)) (unreachable))

        (local.set $len (i32.sub (local.get $len) (i32.load (local.get $written))))
        (local.set $ptr (i32.add (local.get $ptr) (i32.load (local.get $written))))
      end
    )
  )

  (func $strlen (param $ptr i32) (result i32)
    (local $len i32)
    (loop
      (i32.load8_u (i32.add (local.get $ptr) (local.get $len)))
      if
        (local.set $len (i32.add (local.get $len) (i32.const 1)))
        br 1
      end
    )
    local.get $len
  )

  (data (i32.const 10) "\n")
)

