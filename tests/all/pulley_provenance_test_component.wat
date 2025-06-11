(component
  (type $e' (enum "A" "B" "C"))
  (import "host-u32" (func $host-u32 (param "x" u32) (result u32)))
  (import "e" (type $e (eq $e')))
  (import "host-enum" (func $host-enum (param "x" $e) (result $e)))
  (import "host-option" (func $host-option (param "x" (option u8)) (result (option u8))))
  (type $result (result u16 (error s64)))
  (import "host-result" (func $host-result (param "x" $result) (result $result)))
  (import "host-string" (func $host-string (param "x" string) (result string)))
  (import "host-list" (func $host-list (param "x" (list string)) (result (list string))))

  (core module $libc
    (memory (export "memory") 1)
    (global $last (mut i32) (i32.const 8))
    (func $realloc (export "realloc")
        (param $old_ptr i32)
        (param $old_size i32)
        (param $align i32)
        (param $new_size i32)
        (result i32)

        (local $ret i32)

        ;; fail if the old pointer is non-null
        local.get $old_ptr
        if
          unreachable
        end

        ;; align up `$last`
        (global.set $last
            (i32.and
                (i32.add
                    (global.get $last)
                    (i32.add
                        (local.get $align)
                        (i32.const -1)))
                (i32.xor
                    (i32.add
                        (local.get $align)
                        (i32.const -1))
                    (i32.const -1))))

        ;; save the current value of `$last` as the return value
        global.get $last
        local.set $ret

        ;; bump our pointer
        (global.set $last
            (i32.add
                (global.get $last)
                (local.get $new_size)))

        ;; while `memory.size` is less than `$last`, grow memory
        ;; by one page
        (loop $loop
            (if
                (i32.lt_u
                    (i32.mul (memory.size) (i32.const 65536))
                    (global.get $last))
                (then
                    i32.const 1
                    memory.grow
                    ;; test to make sure growth succeeded
                    i32.const -1
                    i32.eq
                    if unreachable end

                    br $loop)))

        local.get $ret
    )
  )
  (core instance $libc (instantiate $libc))

  (core func $host-u32 (canon lower (func $host-u32)))
  (core func $host-enum (canon lower (func $host-enum)))
  (core func $host-option (canon lower (func $host-option) (memory $libc "memory")))
  (core func $host-result (canon lower (func $host-result) (memory $libc "memory")))
  (core func $host-string (canon lower (func $host-string)
    (memory $libc "memory") (realloc (func $libc "realloc"))))
  (core func $host-list (canon lower (func $host-list)
    (memory $libc "memory") (realloc (func $libc "realloc"))))

  (type $a (resource (rep i32)))
  (core func $new-a (canon resource.new $a))
  (core func $drop-a (canon resource.drop $a))

  (core module $m
    (import "" "host-u32" (func $host-u32 (param i32) (result i32)))
    (import "" "host-enum" (func $host-enum (param i32) (result i32)))
    (import "" "host-option" (func $host-option (param i32 i32 i32)))
    (import "" "host-result" (func $host-result (param i32 i64 i32)))
    (import "" "host-string" (func $host-string (param i32 i32 i32)))
    (import "" "host-list" (func $host-list (param i32 i32 i32)))
    (import "" "new-a" (func $new-a (param i32) (result i32)))
    (import "" "drop-a" (func $drop-a (param i32)))

    (func (export "guest-u32") (param i32) (result i32) local.get 0 call $host-u32)
    (func (export "guest-enum") (param i32) (result i32) local.get 0 call $host-enum)
    (func (export "guest-option") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      i32.const 100
      call $host-option
      i32.const 100)
    (func (export "guest-result") (param i32 i64) (result i32)
      local.get 0
      local.get 1
      i32.const 96
      call $host-result
      i32.const 96)
    (func (export "guest-string") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      i32.const 96
      call $host-string
      i32.const 96)
    (func (export "guest-list") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      i32.const 96
      call $host-list
      i32.const 96)

    (func (export "resource-intrinsics")
      (call $drop-a (call $new-a (i32.const 100)))
    )
  )

  (core instance $i (instantiate $m
    (with "libc" (instance $libc))
    (with "" (instance
        (export "host-u32" (func $host-u32))
        (export "host-enum" (func $host-enum))
        (export "host-option" (func $host-option))
        (export "host-result" (func $host-result))
        (export "host-string" (func $host-string))
        (export "host-list" (func $host-list))
        (export "new-a" (func $new-a))
        (export "drop-a" (func $drop-a))
    ))
  ))
  (func (export "guest-u32") (param "x" u32) (result u32)
    (canon lift (core func $i "guest-u32")))
  (func (export "guest-enum") (param "x" $e) (result $e)
    (canon lift (core func $i "guest-enum")))
  (func (export "guest-option") (param "x" (option u8)) (result (option u8))
    (canon lift (core func $i "guest-option") (memory $libc "memory")))
  (func (export "guest-result") (param "x" $result) (result $result)
    (canon lift (core func $i "guest-result") (memory $libc "memory")))
  (func (export "guest-string") (param "x" string) (result string)
    (canon lift (core func $i "guest-string") (memory $libc "memory")
                (realloc (func $libc "realloc"))))
  (func (export "guest-list") (param "x" (list string)) (result (list string))
    (canon lift (core func $i "guest-list") (memory $libc "memory")
                (realloc (func $libc "realloc"))))
  (func (export "resource-intrinsics")
    (canon lift (core func $i "resource-intrinsics") ))

)
