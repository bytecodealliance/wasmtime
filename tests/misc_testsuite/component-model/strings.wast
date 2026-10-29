;;! multi_memory = true

;; Returning an unaligned utf16 string is invalid
(component definition $A
  (core module $m
    (memory (export "m") 1)
    (func (export "f") (result i32)
      (i32.store (i32.const 4) (i32.const 1))
      (i32.store (i32.const 8) (i32.const 0))
      i32.const 4
    )
  )
  (core instance $m (instantiate $m))
  (func (export "f1") (result string)
    (canon lift (core func $m "f") (memory (core memory $m "m")) string-encoding=utf16))
  (func (export "f2") (result string)
    (canon lift (core func $m "f") (memory (core memory $m "m")) string-encoding=latin1+utf16))

)
(component instance $A $A)
(assert_trap (invoke "f1") "string pointer not aligned to 2")
(component instance $A $A)
(assert_trap (invoke "f2") "string pointer not aligned to 2")

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
        (realloc (core func $m "realloc"))
        (memory (core memory $m "memory"))
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
    (core func $f (canon lower (func $f) string-encoding=utf8 (memory (core memory $libc "memory"))))
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

(assert_trap (invoke "f") "unaligned pointer")

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
        (realloc (core func $m "realloc"))
        (memory (core memory $m "memory"))
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
    (core func $f (canon lower (func $f) string-encoding=utf16 (memory (core memory $libc "memory"))))
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
(assert_trap (invoke "f") "unaligned pointer")

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
        (realloc (core func $m "realloc"))
        (memory (core memory $m "memory"))
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
    (core func $f (canon lower (func $f) string-encoding=latin1+utf16 (memory (core memory $libc "memory"))))
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
(assert_trap (invoke "f") "unaligned pointer")

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
        (realloc (core func $m "realloc"))
        (memory (core memory $m "memory"))
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
    (core func $f (canon lower (func $f) (memory (core memory $libc "memory"))))
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
(assert_trap (invoke "f") "unaligned pointer")

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
        (realloc (core func $m "realloc"))
        (memory (core memory $m "memory"))
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
    (core func $f (canon lower (func $f) (memory (core memory $libc "memory"))))
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
(assert_trap (invoke "f") "unaligned pointer")

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
        (realloc (core func $m "realloc"))
        (memory (core memory $m "memory"))
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
    (core func $f (canon lower (func $f) (memory (core memory $libc "memory"))))
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
(assert_trap (invoke "f") "string content out-of-bounds")

;; The following tests cover transcoding between each pair of string
;; encodings. `$dst-*` components receive a string with the given encoding
;; and `$src-*` components pass a string from their own memory with the given
;; encoding. Each pair is exported as `<src>-to-<dst>`, where `compact` is
;; latin1+utf16.

;; Strings which are out-of-bounds, or whose `ptr + len` calculation
;; overflows, trap.
(component definition $Strings
  (component $dst-utf8
    (core module $m
      (func (export "f") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        i32.const 0)
      (memory (export "memory") 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" string)
      (canon lift (core func $i "f") (realloc (core func $i "realloc"))
        (memory (core memory $i "memory")) string-encoding=utf8))
  )
  (component $dst-utf16
    (core module $m
      (func (export "f") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        i32.const 0)
      (memory (export "memory") 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" string)
      (canon lift (core func $i "f") (realloc (core func $i "realloc"))
        (memory (core memory $i "memory")) string-encoding=utf16))
  )
  (component $dst-compact
    (core module $m
      (func (export "f") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        i32.const 0)
      (memory (export "memory") 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" string)
      (canon lift (core func $i "f") (realloc (core func $i "realloc"))
        (memory (core memory $i "memory")) string-encoding=latin1+utf16))
  )
  (component $src-utf8
    (import "f" (func $f (param "a" string)))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding=utf8
      (memory (core memory $libc "memory"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (func (export "f") (param i32 i32)
        (call $f (local.get 0) (local.get 1)))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f))))
    ))
    (func (export "f") (param "ptr" u32) (param "len" u32)
      (canon lift (core func $i "f")))
  )
  (component $src-utf16
    (import "f" (func $f (param "a" string)))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding=utf16
      (memory (core memory $libc "memory"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (func (export "f") (param i32 i32)
        (call $f (local.get 0) (local.get 1)))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f))))
    ))
    (func (export "f") (param "ptr" u32) (param "len" u32)
      (canon lift (core func $i "f")))
  )
  (component $src-compact
    (import "f" (func $f (param "a" string)))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding=latin1+utf16
      (memory (core memory $libc "memory"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (func (export "f") (param i32 i32)
        (call $f (local.get 0) (local.get 1)))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f))))
    ))
    (func (export "f") (param "ptr" u32) (param "len" u32)
      (canon lift (core func $i "f")))
  )

  (instance $dst-utf8 (instantiate $dst-utf8))
  (instance $dst-utf16 (instantiate $dst-utf16))
  (instance $dst-compact (instantiate $dst-compact))
  (instance $utf8-to-utf8 (instantiate $src-utf8
    (with "f" (func $dst-utf8 "f"))))
  (export "utf8-to-utf8" (func $utf8-to-utf8 "f"))
  (instance $utf8-to-utf16 (instantiate $src-utf8
    (with "f" (func $dst-utf16 "f"))))
  (export "utf8-to-utf16" (func $utf8-to-utf16 "f"))
  (instance $utf8-to-compact (instantiate $src-utf8
    (with "f" (func $dst-compact "f"))))
  (export "utf8-to-compact" (func $utf8-to-compact "f"))
  (instance $utf16-to-utf8 (instantiate $src-utf16
    (with "f" (func $dst-utf8 "f"))))
  (export "utf16-to-utf8" (func $utf16-to-utf8 "f"))
  (instance $utf16-to-utf16 (instantiate $src-utf16
    (with "f" (func $dst-utf16 "f"))))
  (export "utf16-to-utf16" (func $utf16-to-utf16 "f"))
  (instance $utf16-to-compact (instantiate $src-utf16
    (with "f" (func $dst-compact "f"))))
  (export "utf16-to-compact" (func $utf16-to-compact "f"))
  (instance $compact-to-utf8 (instantiate $src-compact
    (with "f" (func $dst-utf8 "f"))))
  (export "compact-to-utf8" (func $compact-to-utf8 "f"))
  (instance $compact-to-utf16 (instantiate $src-compact
    (with "f" (func $dst-utf16 "f"))))
  (export "compact-to-utf16" (func $compact-to-utf16 "f"))
  (instance $compact-to-compact (instantiate $src-compact
    (with "f" (func $dst-compact "f"))))
  (export "compact-to-compact" (func $compact-to-compact "f"))
)
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-utf8" (u32.const 0x80000000) (u32.const 0x0))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-utf8" (u32.const 0x80000000) (u32.const 0x1))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-utf16" (u32.const 0x80000000) (u32.const 0x0))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-utf16" (u32.const 0x80000000) (u32.const 0x1))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-compact" (u32.const 0x80000000) (u32.const 0x0))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-compact" (u32.const 0x80000000) (u32.const 0x1))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-utf8" (u32.const 0x80000000) (u32.const 0x0))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-utf8" (u32.const 0x80000000) (u32.const 0x1))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-utf16" (u32.const 0x80000000) (u32.const 0x0))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-utf16" (u32.const 0x80000000) (u32.const 0x1))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-compact" (u32.const 0x80000000) (u32.const 0x0))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-compact" (u32.const 0x80000000) (u32.const 0x1))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-utf8" (u32.const 0x80000000) (u32.const 0x0))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-utf8" (u32.const 0x80000000) (u32.const 0x1))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-utf16" (u32.const 0x80000000) (u32.const 0x0))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-utf16" (u32.const 0x80000000) (u32.const 0x1))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-compact"
  (u32.const 0x80000000) (u32.const 0x0))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-compact"
  (u32.const 0x80000000) (u32.const 0x1))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-utf8" (u32.const 0x3e8) (u32.const 0x80000000))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-utf8" (u32.const 0x3e8) (u32.const 0x7fffff9c))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-utf16" (u32.const 0x3e8) (u32.const 0x80000000))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-utf16" (u32.const 0x3e8) (u32.const 0x40000000))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-utf16" (u32.const 0x3e8) (u32.const 0x3fffff9c))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-compact" (u32.const 0x3e8) (u32.const 0x80000000))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf8-to-compact" (u32.const 0x3e8) (u32.const 0x7fffff9c))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-utf8" (u32.const 0x3e8) (u32.const 0x40000000))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-utf8" (u32.const 0x3e8) (u32.const 0x3fffff9c))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-utf16" (u32.const 0x3e8) (u32.const 0x40000000))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-utf16" (u32.const 0x3e8) (u32.const 0x3fffff9c))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-compact"
  (u32.const 0x3e8) (u32.const 0x40000000))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "utf16-to-compact"
  (u32.const 0x3e8) (u32.const 0x3fffff9c))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-utf8" (u32.const 0x3e8) (u32.const 0xc0000000))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-utf8" (u32.const 0x3e8) (u32.const 0xbfffff9c))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-utf16"
  (u32.const 0x3e8) (u32.const 0xc0000000))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-utf16"
  (u32.const 0x3e8) (u32.const 0xbfffff9c))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-compact"
  (u32.const 0x3e8) (u32.const 0xc0000000))
  "string content out-of-bounds")
(component instance $Strings $Strings)
(assert_trap (invoke "compact-to-compact"
  (u32.const 0x3e8) (u32.const 0xbfffff9c))
  "string content out-of-bounds")

;; The pointer returned from `realloc` is bounds-checked.
(component definition $ReallocOob
  (component $dst-utf8
    (core module $m
      (func (export "f") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        i32.const 100000)
      (memory (export "memory") 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" string)
      (canon lift (core func $i "f") (realloc (core func $i "realloc"))
        (memory (core memory $i "memory")) string-encoding=utf8))
  )
  (component $dst-utf16
    (core module $m
      (func (export "f") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        i32.const 100000)
      (memory (export "memory") 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" string)
      (canon lift (core func $i "f") (realloc (core func $i "realloc"))
        (memory (core memory $i "memory")) string-encoding=utf16))
  )
  (component $dst-compact
    (core module $m
      (func (export "f") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        i32.const 100000)
      (memory (export "memory") 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" string)
      (canon lift (core func $i "f") (realloc (core func $i "realloc"))
        (memory (core memory $i "memory")) string-encoding=latin1+utf16))
  )
  (component $src-utf8
    (import "f" (func $f (param "a" string)))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding=utf8
      (memory (core memory $libc "memory"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (func (export "f") (param i32 i32)
        (call $f (local.get 0) (local.get 1)))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f))))
    ))
    (func (export "f") (param "ptr" u32) (param "len" u32)
      (canon lift (core func $i "f")))
  )
  (component $src-utf16
    (import "f" (func $f (param "a" string)))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding=utf16
      (memory (core memory $libc "memory"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (func (export "f") (param i32 i32)
        (call $f (local.get 0) (local.get 1)))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f))))
    ))
    (func (export "f") (param "ptr" u32) (param "len" u32)
      (canon lift (core func $i "f")))
  )
  (component $src-compact
    (import "f" (func $f (param "a" string)))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) string-encoding=latin1+utf16
      (memory (core memory $libc "memory"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (func (export "f") (param i32 i32)
        (call $f (local.get 0) (local.get 1)))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f))))
    ))
    (func (export "f") (param "ptr" u32) (param "len" u32)
      (canon lift (core func $i "f")))
  )

  (instance $dst-utf8 (instantiate $dst-utf8))
  (instance $dst-utf16 (instantiate $dst-utf16))
  (instance $dst-compact (instantiate $dst-compact))
  (instance $utf8-to-utf8 (instantiate $src-utf8
    (with "f" (func $dst-utf8 "f"))))
  (export "utf8-to-utf8" (func $utf8-to-utf8 "f"))
  (instance $utf8-to-utf16 (instantiate $src-utf8
    (with "f" (func $dst-utf16 "f"))))
  (export "utf8-to-utf16" (func $utf8-to-utf16 "f"))
  (instance $utf8-to-compact (instantiate $src-utf8
    (with "f" (func $dst-compact "f"))))
  (export "utf8-to-compact" (func $utf8-to-compact "f"))
  (instance $utf16-to-utf8 (instantiate $src-utf16
    (with "f" (func $dst-utf8 "f"))))
  (export "utf16-to-utf8" (func $utf16-to-utf8 "f"))
  (instance $utf16-to-utf16 (instantiate $src-utf16
    (with "f" (func $dst-utf16 "f"))))
  (export "utf16-to-utf16" (func $utf16-to-utf16 "f"))
  (instance $utf16-to-compact (instantiate $src-utf16
    (with "f" (func $dst-compact "f"))))
  (export "utf16-to-compact" (func $utf16-to-compact "f"))
  (instance $compact-to-utf8 (instantiate $src-compact
    (with "f" (func $dst-utf8 "f"))))
  (export "compact-to-utf8" (func $compact-to-utf8 "f"))
  (instance $compact-to-utf16 (instantiate $src-compact
    (with "f" (func $dst-utf16 "f"))))
  (export "compact-to-utf16" (func $compact-to-utf16 "f"))
  (instance $compact-to-compact (instantiate $src-compact
    (with "f" (func $dst-compact "f"))))
  (export "compact-to-compact" (func $compact-to-compact "f"))
)
(component instance $ReallocOob $ReallocOob)
(assert_trap (invoke "utf8-to-utf8" (u32.const 0x3e8) (u32.const 0xa))
  "string content out-of-bounds")
(component instance $ReallocOob $ReallocOob)
(assert_trap (invoke "utf8-to-utf16" (u32.const 0x3e8) (u32.const 0xa))
  "string content out-of-bounds")
(component instance $ReallocOob $ReallocOob)
(assert_trap (invoke "utf8-to-compact" (u32.const 0x3e8) (u32.const 0xa))
  "string content out-of-bounds")
(component instance $ReallocOob $ReallocOob)
(assert_trap (invoke "utf16-to-utf8" (u32.const 0x3e8) (u32.const 0xa))
  "string content out-of-bounds")
(component instance $ReallocOob $ReallocOob)
(assert_trap (invoke "utf16-to-utf16" (u32.const 0x3e8) (u32.const 0xa))
  "string content out-of-bounds")
(component instance $ReallocOob $ReallocOob)
(assert_trap (invoke "utf16-to-compact" (u32.const 0x3e8) (u32.const 0xa))
  "string content out-of-bounds")
(component instance $ReallocOob $ReallocOob)
(assert_trap (invoke "compact-to-utf8" (u32.const 0x3e8) (u32.const 0xa))
  "string content out-of-bounds")
(component instance $ReallocOob $ReallocOob)
(assert_trap (invoke "compact-to-utf16" (u32.const 0x3e8) (u32.const 0xa))
  "string content out-of-bounds")
(component instance $ReallocOob $ReallocOob)
(assert_trap (invoke "compact-to-compact" (u32.const 0x3e8) (u32.const 0xa))
  "string content out-of-bounds")

;; Strings which are invalid in their source encoding trap when transcoded.
(component definition $RawBytes
  ;; The raw bytes passed by the `$src-*` components, at fixed offsets.
  (core module $data
    (memory (export "memory") 1)
    (data (i32.const 0x100) "\ff")
    (data (i32.const 0x200) "valid string until \ffthen valid again")
    (data (i32.const 0x300) "symbol \ce\a3 until \ffthen valid")
    (data (i32.const 0x400) "\01\d8")
    (data (i32.const 0x500) "\ff\ff\01\d8")
    (data (i32.const 0x600) "\ab\00\ff\ff\01\d8")
    (data (i32.const 0x700) "\ab\00\ff\00")
  )

  (component $dst-utf8
    (core module $m
      (func (export "f") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        i32.const 0)
      (memory (export "memory") 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" string)
      (canon lift (core func $i "f") (realloc (core func $i "realloc"))
        (memory (core memory $i "memory")) string-encoding=utf8))
  )
  (component $dst-utf16
    (core module $m
      (func (export "f") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        i32.const 0)
      (memory (export "memory") 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" string)
      (canon lift (core func $i "f") (realloc (core func $i "realloc"))
        (memory (core memory $i "memory")) string-encoding=utf16))
  )
  (component $dst-compact
    (core module $m
      (func (export "f") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        i32.const 0)
      (memory (export "memory") 1)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" string)
      (canon lift (core func $i "f") (realloc (core func $i "realloc"))
        (memory (core memory $i "memory")) string-encoding=latin1+utf16))
  )
  (component $src-utf8
    (import "f" (func $f (param "a" string)))
    (alias outer $RawBytes $data (core module $data))
    (core instance $data (instantiate $data))
    (core func $f (canon lower (func $f) string-encoding=utf8
      (memory (core memory $data "memory"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (func (export "f") (param i32 i32)
        (call $f (local.get 0) (local.get 1)))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f))))
    ))
    (func (export "f") (param "ptr" u32) (param "len" u32)
      (canon lift (core func $i "f")))
  )
  (component $src-utf16
    (import "f" (func $f (param "a" string)))
    (alias outer $RawBytes $data (core module $data))
    (core instance $data (instantiate $data))
    (core func $f (canon lower (func $f) string-encoding=utf16
      (memory (core memory $data "memory"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (func (export "f") (param i32 i32)
        (call $f (local.get 0) (local.get 1)))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f))))
    ))
    (func (export "f") (param "ptr" u32) (param "len" u32)
      (canon lift (core func $i "f")))
  )
  (component $src-compact
    (import "f" (func $f (param "a" string)))
    (alias outer $RawBytes $data (core module $data))
    (core instance $data (instantiate $data))
    (core func $f (canon lower (func $f) string-encoding=latin1+utf16
      (memory (core memory $data "memory"))))
    (core module $m
      (import "" "f" (func $f (param i32 i32)))
      (func (export "f") (param i32 i32)
        (call $f (local.get 0) (local.get 1)))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f))))
    ))
    (func (export "f") (param "ptr" u32) (param "len" u32)
      (canon lift (core func $i "f")))
  )

  (instance $dst-utf8 (instantiate $dst-utf8))
  (instance $dst-utf16 (instantiate $dst-utf16))
  (instance $dst-compact (instantiate $dst-compact))
  (instance $utf8-to-utf8 (instantiate $src-utf8
    (with "f" (func $dst-utf8 "f"))))
  (export "utf8-to-utf8" (func $utf8-to-utf8 "f"))
  (instance $utf8-to-utf16 (instantiate $src-utf8
    (with "f" (func $dst-utf16 "f"))))
  (export "utf8-to-utf16" (func $utf8-to-utf16 "f"))
  (instance $utf8-to-compact (instantiate $src-utf8
    (with "f" (func $dst-compact "f"))))
  (export "utf8-to-compact" (func $utf8-to-compact "f"))
  (instance $utf16-to-utf8 (instantiate $src-utf16
    (with "f" (func $dst-utf8 "f"))))
  (export "utf16-to-utf8" (func $utf16-to-utf8 "f"))
  (instance $utf16-to-utf16 (instantiate $src-utf16
    (with "f" (func $dst-utf16 "f"))))
  (export "utf16-to-utf16" (func $utf16-to-utf16 "f"))
  (instance $utf16-to-compact (instantiate $src-utf16
    (with "f" (func $dst-compact "f"))))
  (export "utf16-to-compact" (func $utf16-to-compact "f"))
  (instance $compact-to-utf8 (instantiate $src-compact
    (with "f" (func $dst-utf8 "f"))))
  (export "compact-to-utf8" (func $compact-to-utf8 "f"))
  (instance $compact-to-utf16 (instantiate $src-compact
    (with "f" (func $dst-utf16 "f"))))
  (export "compact-to-utf16" (func $compact-to-utf16 "f"))
  (instance $compact-to-compact (instantiate $src-compact
    (with "f" (func $dst-compact "f"))))
  (export "compact-to-compact" (func $compact-to-compact "f"))
)
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "utf8-to-utf8" (u32.const 0x100) (u32.const 1))
  "invalid utf8 encoding")
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "utf8-to-utf8" (u32.const 0x200) (u32.const 36))
  "invalid utf8 encoding")
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "utf8-to-utf16" (u32.const 0x200) (u32.const 36))
  "invalid utf8 encoding")
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "utf8-to-utf8" (u32.const 0x300) (u32.const 27))
  "invalid utf8 encoding")
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "utf8-to-utf16" (u32.const 0x300) (u32.const 27))
  "invalid utf8 encoding")
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "utf8-to-compact" (u32.const 0x300) (u32.const 27))
  "invalid utf8 encoding")
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "utf16-to-utf8" (u32.const 0x400) (u32.const 1))
  "invalid utf16 encoding")
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "utf16-to-utf16" (u32.const 0x400) (u32.const 1))
  "invalid utf16 encoding")
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "utf16-to-compact" (u32.const 0x500) (u32.const 2))
  "invalid utf16 encoding")

;; A latin1+utf16 length with the high bit set is a count of utf16 code units.
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "compact-to-utf8" (u32.const 0x400) (u32.const 0x80000001))
  "invalid utf16 encoding")
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "compact-to-utf16" (u32.const 0x400) (u32.const 0x80000001))
  "invalid utf16 encoding")
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "compact-to-utf16" (u32.const 0x500) (u32.const 0x80000002))
  "invalid utf16 encoding")
(component instance $RawBytes $RawBytes)
(assert_trap
  (invoke "compact-to-compact" (u32.const 0x600) (u32.const 0x80000003))
  "invalid utf16 encoding")

;; This latin1+utf16 string is valid and gets compressed to latin1.
(component instance $RawBytes $RawBytes)
(assert_return
  (invoke "compact-to-compact" (u32.const 0x700) (u32.const 0x80000002)))
