(module
  (import "wasi_snapshot_preview1" "fd_read"
    (func $read (param i32 i32 i32 i32) (result i32)))

  (memory (export "memory") 1)

  (func (export "count") (result i32)
    (call $count-up-to (i32.const -1))
  )

  (func $count-up-to (export "count-up-to") (param $up-to i32) (result i32)
    (local $size i32)

    (i32.eqz (local.get $up-to))
    if
      local.get 0
      return
    end
    loop $the-loop
      ;; setup a basic ciovec pointing into memory
      (i32.store
        (i32.const 100)
        (i32.const 200))
      (i32.store
        (i32.const 104)
        (i32.const 1000))


      (call $read
        (i32.const 0)       ;; stdin fileno
        (i32.const 100)     ;; ciovec base
        (i32.const 1)       ;; ciovec len
        (i32.const 8)       ;; ret val ptr
      )
      ;; reading stdin must succeed (e.g. return 0)
      if unreachable end

      ;; update with how many bytes were read
      (local.set $size
        (i32.add
          (local.get $size)
          (i32.load (i32.const 8))))


      ;; if no data was read, exit the loop
      ;; if the size read exceeds what we're supposed to read, also exit the
      ;; loop
      (i32.load (i32.const 8))
      if
        (i32.lt_u (local.get $size) (local.get $up-to))
        if
          br $the-loop
        end
      end
    end

    local.get $size
  )
)
