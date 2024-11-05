;;! multi_memory = true

;; unaligned utf16 string
(assert_trap
  (component
    (component $c
      (core module $m
        (func (export "") (param i32 i32))
        (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
        (memory (export "memory") 1)
      )
      (core instance $m (instantiate $m))
      (func (export "a") (param "a" string)
        (canon lift (core func $m "") (realloc (func $m "realloc")) (memory $m "memory"))
      )
    )

    (component $c2
      (import "a" (func $f (param "a" string)))
      (core module $libc
        (memory (export "memory") 1)
      )
      (core instance $libc (instantiate $libc))
      (core func $f (canon lower (func $f) string-encoding=utf16 (memory $libc "memory")))
      (core module $m
        (import "" "" (func $f (param i32 i32)))

        (func $start (call $f (i32.const 1) (i32.const 0)))
        (start $start)
      )
      (core instance (instantiate $m (with "" (instance (export "" (func $f))))))
    )

    (instance $c (instantiate $c))
    (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  )
  "unreachable")

;; unaligned latin1+utf16 string, even with the latin1 encoding
(assert_trap
  (component
    (component $c
      (core module $m
        (func (export "") (param i32 i32))
        (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
        (memory (export "memory") 1)
      )
      (core instance $m (instantiate $m))
      (func (export "a") (param "a" string)
        (canon lift (core func $m "") (realloc (func $m "realloc")) (memory $m "memory"))
      )
    )

    (component $c2
      (import "a" (func $f (param "a" string)))
      (core module $libc
        (memory (export "memory") 1)
      )
      (core instance $libc (instantiate $libc))
      (core func $f (canon lower (func $f) string-encoding=latin1+utf16 (memory $libc "memory")))
      (core module $m
        (import "" "" (func $f (param i32 i32)))

        (func $start (call $f (i32.const 1) (i32.const 0)))
        (start $start)
      )
      (core instance (instantiate $m (with "" (instance (export "" (func $f))))))
    )

    (instance $c (instantiate $c))
    (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  )
  "unreachable")

;; out of bounds utf8->utf8 string
(assert_trap
  (component
    (component $c
      (core module $m
        (func (export "") (param i32 i32))
        (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
        (memory (export "memory") 1)
      )
      (core instance $m (instantiate $m))
      (func (export "a") (param "a" string)
        (canon lift (core func $m "") (realloc (func $m "realloc")) (memory $m "memory")
          string-encoding=utf8)
      )
    )

    (component $c2
      (import "a" (func $f (param "a" string)))
      (core module $libc
        (memory (export "memory") 1)
      )
      (core instance $libc (instantiate $libc))
      (core func $f (canon lower (func $f) string-encoding=utf8 (memory $libc "memory")))
      (core module $m
        (import "" "" (func $f (param i32 i32)))

        (func $start (call $f (i32.const 0x8000_0000) (i32.const 1)))
        (start $start)
      )
      (core instance (instantiate $m (with "" (instance (export "" (func $f))))))
    )

    (instance $c (instantiate $c))
    (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  )
  "unreachable")
