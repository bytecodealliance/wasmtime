(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))

  (import "wasi_snapshot_preview1" "environ_get"
    (func $environ_get (param i32 i32) (result i32)))

  (memory (export "memory") 1)

  (func (export "_start")
    (local $envptrs i32)
    (local $envmem i32)
    (local $env i32)

    (local.set $envptrs (i32.mul (memory.grow (i32.const 1)) (i32.const 65536)))
    (local.set $envmem (i32.mul (memory.grow (i32.const 1)) (i32.const 65536)))

    (if (i32.ne
          (call $environ_get (local.get $envptrs) (local.get $envmem))
          (i32.const 0))
        (unreachable))

    (loop
      (local.set $env (i32.load (local.get $envptrs)))
      (local.set $envptrs (i32.add (local.get $envptrs) (i32.const 4)))
      (if (i32.eq (local.get $env) (i32.const 0)) (return))

      (call $write_all (local.get $env) (call $strlen (local.get $env)))
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


