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

;; utf8 -> utf16 -- when shrinking memory it must be aligned
(component
  (component $c
    (core module $m
      (func (export "") (param i32 i32) unreachable)
      (func (export "realloc") (param $old_ptr i32) (param $old_size i32)
                               (param $align i32) (param $new_size i32) (result i32)
        (if (i32.ne (local.get $align) (i32.const 2)) (then unreachable))
        (if (result i32) (i32.eqz (local.get $old_ptr))
          (then (i32.const 2)) ;; first allocation aligned
          (else (i32.const 3)) ;; second allocation unaligned
        )
      )
      (memory (export "memory") 1)
    )
    (core instance $m (instantiate $m))
    (func (export "a") (param "a" string)
      (canon lift
        (core func $m "")
        (realloc (func $m "realloc"))
        (memory $m "memory")
        string-encoding=utf16)
    )
  )

  (component $c2
    (import "a" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
      ;; "àà" is  2 UTF-16 code units (4 bytes), and 4 bytes in UTF-8
      ;; Pessimistic alloc = 4 * 2 = 8 bytes, shrinks to 4 bytes after.
      (data (memory 0) (i32.const 0) "àà")
    )
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding=utf8 (memory $libc "memory")))
    (core module $m
      (import "" "" (func $f (param i32 i32)))
      (func (export "f") (call $f (i32.const 0) (i32.const 4)))
    )
    (core instance $m (instantiate $m (with "" (instance (export "" (func $f))))))
    (func (export "f") (canon lift (core func $m "f")))
  )

  (instance $c (instantiate $c))
  (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  (export "f" (func $c2 "f"))
)

(assert_trap (invoke "f") "unreachable")

;; utf16 -> latin1+utf16 -- when shrinking memory it must be aligned
(component
  (component $c
    (core module $m
      (func (export "") (param i32 i32))
      (func (export "realloc") (param $old_ptr i32) (param $old_size i32)
                               (param $align i32) (param $new_size i32) (result i32)
        (if (i32.ne (local.get $align) (i32.const 2)) (then unreachable))
        (if (result i32) (i32.eqz (local.get $old_ptr))
          (then (i32.const 2)) ;; first allocation aligned
          (else (i32.const 3)) ;; second allocation unaligned
        )
      )
      (memory (export "memory") 1)
    )
    (core instance $m (instantiate $m))
    (func (export "a") (param "a" string)
      (canon lift
        (core func $m "")
        (realloc (func $m "realloc"))
        (memory $m "memory")
        string-encoding=latin1+utf16)
    )
  )

  (component $c2
    (import "a" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
      ;; "AΣ" in UTF-16: 0x41 0x00 0xA3 0x03 (Σ = U+03A3, not Latin-1)
      ;; Forces transcoding to take the UTF-16 grow path.
      (data (memory 0) (i32.const 0) "\41\00\a3\03")
    )
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding=utf16 (memory $libc "memory")))
    (core module $m
      (import "" "" (func $f (param i32 i32)))
      (func (export "f") (call $f (i32.const 0) (i32.const 2)))
    )
    (core instance $m (instantiate $m (with "" (instance (export "" (func $f))))))
    (func (export "f") (canon lift (core func $m "f")))
  )

  (instance $c (instantiate $c))
  (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  (export "f" (func $c2 "f"))
)
(assert_trap (invoke "f") "unreachable")

;; latin1+utf16 -> latin1+utf16 -- auto-downsize
(component
  (component $c
    (core module $m
      (func (export "") (param i32 i32) unreachable)
      (func (export "realloc") (param $old_ptr i32) (param $old_size i32)
                               (param $align i32) (param $new_size i32) (result i32)
        (if (i32.ne (local.get $align) (i32.const 2)) (then unreachable))
        (if (result i32) (i32.eqz (local.get $old_ptr))
          (then (i32.const 2)) ;; first allocation aligned
          (else (i32.const 3)) ;; second allocation unaligned
        )
      )
      (memory (export "memory") 1)
    )
    (core instance $m (instantiate $m))
    (func (export "a") (param "a" string)
      (canon lift
        (core func $m "")
        (realloc (func $m "realloc"))
        (memory $m "memory")
        string-encoding=latin1+utf16)
    )
  )

  (component $c2
    (import "a" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
      ;; "AA" in UTF-16: 0x41 0x00 0x41 0x00
      (data (memory 0) (i32.const 0) "\41\00\41\00")
    )
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding=latin1+utf16 (memory $libc "memory")))
    (core module $m
      (import "" "" (func $f (param i32 i32)))
      ;; the length here contains `UTF16_TAG` and it's additionally 1 code
      ;; unit. This is a utf-16 encoded string but during transcoding it'll
      ;; get shrunk to latin 1
      (func (export "f") (call $f (i32.const 0) (i32.const 0x8000_0002)))
    )
    (core instance $m (instantiate $m (with "" (instance (export "" (func $f))))))
    (func (export "f") (canon lift (core func $m "f")))
  )

  (instance $c (instantiate $c))
  (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  (export "f" (func $c2 "f"))
)
(assert_trap (invoke "f") "unreachable")

;; utf8 -> latin1+utf16 -- initial encode finishes but needs downsizing
(component
  (component $c
    (core module $m
      (func (export "") (param i32 i32) unreachable)
      (func (export "realloc") (param $old_ptr i32) (param $old_size i32)
                               (param $align i32) (param $new_size i32) (result i32)
        (if (i32.ne (local.get $align) (i32.const 2)) (then unreachable))
        (if (result i32) (i32.eqz (local.get $old_ptr))
          (then (i32.const 2)) ;; first allocation aligned
          (else (i32.const 3)) ;; second allocation unaligned
        )
      )
      (memory (export "memory") 1)
    )
    (core instance $m (instantiate $m))
    (func (export "a") (param "a" string)
      (canon lift
        (core func $m "")
        (realloc (func $m "realloc"))
        (memory $m "memory")
        string-encoding=latin1+utf16)
    )
  )

  (component $c2
    (import "a" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
      ;; "Ë" in UTF-8 is "\xc3\xab", which is 2 bytes, but in latin1+utf16 it's
      ;; 1 byte (0xCB). The initial allocation of 2 bytes completes the entire
      ;; transcode but the final allocation needs to be shrunk to 1 byte.
      (data (memory 0) (i32.const 0) "Ë")
    )
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) (memory $libc "memory")))
    (core module $m
      (import "" "" (func $f (param i32 i32)))
      (func (export "f") (call $f (i32.const 0) (i32.const 2)))
    )
    (core instance $m (instantiate $m (with "" (instance (export "" (func $f))))))
    (func (export "f") (canon lift (core func $m "f")))
  )

  (instance $c (instantiate $c))
  (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  (export "f" (func $c2 "f"))
)
(assert_trap (invoke "f") "unreachable")

;; utf8 -> latin1+utf16
;;  - first realloc fails to hold latin1
;;  - second realloc is too big
;;  - third realloc shrinks
(component
  (component $c
    (core module $m
      (global $cnt (mut i32) (i32.const 0))
      (func (export "") (param i32 i32)
        unreachable
      )
      (func (export "realloc") (param $old_ptr i32) (param $old_size i32)
                               (param $align i32) (param $new_size i32) (result i32)
        (if (i32.ne (local.get $align) (i32.const 2)) (then unreachable))
        (global.set $cnt (i32.add (global.get $cnt) (i32.const 1)))

        ;; first allocation is aligned
        (if (i32.eq (global.get $cnt) (i32.const 1))
          (then
            (if (i32.ne (local.get $old_ptr) (i32.const 0)) (then unreachable))
            (if (i32.ne (local.get $old_size) (i32.const 0)) (then unreachable))
            (if (i32.ne (local.get $new_size) (i32.const 5)) (then unreachable))
            (return (i32.const 2)))
        )
        ;; second allocation is aligned
        (if (i32.eq (global.get $cnt) (i32.const 2))
          (then
            (if (i32.ne (local.get $old_ptr) (i32.const 2)) (then unreachable))
            (if (i32.ne (local.get $old_size) (i32.const 5)) (then unreachable))
            (if (i32.ne (local.get $new_size) (i32.const 10)) (then unreachable))
            (return (i32.const 4)))
        )
        ;; third allocation is unaligned
        (if (i32.eq (global.get $cnt) (i32.const 3))
          (then
            (if (i32.ne (local.get $old_ptr) (i32.const 4)) (then unreachable))
            (if (i32.ne (local.get $old_size) (i32.const 10)) (then unreachable))
            (if (i32.ne (local.get $new_size) (i32.const 4)) (then unreachable))
            (return (i32.const 3)))
        )

        unreachable
      )
      (memory (export "memory") 1)
    )
    (core instance $m (instantiate $m))
    (func (export "a") (param "a" string)
      (canon lift
        (core func $m "")
        (realloc (func $m "realloc"))
        (memory $m "memory")
        string-encoding=latin1+utf16)
    )
  )

  (component $c2
    (import "a" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
      ;; "Ë┛" in UTF-8 is "\xc3\xab\xe2\x8c\x9b", 5 bytes.
      ;; * First, a 5-byte allocation is made to see if it fits in latin 1.
      ;; * This fails since "┛" does not fit in latin1. The second allocation
      ;;   is over-large at 10 bytes (twice the original length).
      ;; * The string encoded in UTF-16 is "\xcb\x00\x1b%", which is 4 bytes.
      ;; * The 10-byte allocation is shrunk to 4 bytes, which is what this
      ;;   test is looking for (proper alignment in the 3rd realloc).
      (data (memory 0) (i32.const 0) "Ë┛")
    )
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) (memory $libc "memory")))
    (core module $m
      (import "" "" (func $f (param i32 i32)))
      (func (export "f") (call $f (i32.const 0) (i32.const 5)))
    )
    (core instance $m (instantiate $m (with "" (instance (export "" (func $f))))))
    (func (export "f") (canon lift (core func $m "f")))
  )

  (instance $c (instantiate $c))
  (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  (export "f" (func $c2 "f"))
)
(assert_trap (invoke "f") "unreachable")

;; utf8 -> latin1+utf16
;;  - first realloc fails to hold latin1
;;  - second realloc is out of bounds
(component
  (component $c
    (core module $m
      (global $cnt (mut i32) (i32.const 0))
      (func (export "") (param i32 i32)
        unreachable
      )
      (func (export "realloc") (param $old_ptr i32) (param $old_size i32)
                               (param $align i32) (param $new_size i32) (result i32)
        (if (i32.ne (local.get $align) (i32.const 2)) (then unreachable))
        (global.set $cnt (i32.add (global.get $cnt) (i32.const 1)))

        ;; first allocation is aligned
        (if (i32.eq (global.get $cnt) (i32.const 1))
          (then
            (if (i32.ne (local.get $old_ptr) (i32.const 0)) (then unreachable))
            (if (i32.ne (local.get $old_size) (i32.const 0)) (then unreachable))
            (if (i32.ne (local.get $new_size) (i32.const 5)) (then unreachable))
            (return (i32.const 2)))
        )
        ;; second allocation is out of bounds
        (if (i32.eq (global.get $cnt) (i32.const 2))
          (then
            (if (i32.ne (local.get $old_ptr) (i32.const 2)) (then unreachable))
            (if (i32.ne (local.get $old_size) (i32.const 5)) (then unreachable))
            (if (i32.ne (local.get $new_size) (i32.const 10)) (then unreachable))
            (return (i32.const -2)))
        )

        unreachable
      )
      (memory (export "memory") 1)
    )
    (core instance $m (instantiate $m))
    (func (export "a") (param "a" string)
      (canon lift
        (core func $m "")
        (realloc (func $m "realloc"))
        (memory $m "memory")
        string-encoding=latin1+utf16)
    )
  )

  (component $c2
    (import "a" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
      ;; "Ë┛" in UTF-8 is "\xc3\xab\xe2\x8c\x9b", 5 bytes.
      ;; * First, a 5-byte allocation is made to see if it fits in latin 1.
      ;; * This fails since "┛" does not fit in latin1. The second allocation
      ;;   is then out of bounds and should trap
      (data (memory 0) (i32.const 0) "Ë┛")
    )
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) (memory $libc "memory")))
    (core module $m
      (import "" "" (func $f (param i32 i32)))
      (func (export "f") (call $f (i32.const 0) (i32.const 5)))
    )
    (core instance $m (instantiate $m (with "" (instance (export "" (func $f))))))
    (func (export "f") (canon lift (core func $m "f")))
  )

  (instance $c (instantiate $c))
  (instance $c2 (instantiate $c2 (with "a" (func $c "a"))))
  (export "f" (func $c2 "f"))
)
(assert_trap (invoke "f") "unreachable")
