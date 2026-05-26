;;! multi_memory = true
;;! hogs_memory = true

;; This is a test which exercises the various behaviors of sending massive
;; strings from one component to another. This ensures that all memory accesses
;; are bounds checked, for example. This additionally ensures that all maximal
;; widths of strings are respected.
;;
;; The test here is relatively carefully crafted to not actually need to create
;; massive strings at runtime. The goal here is to test what kind of trap
;; happens before any "real" transcoding happens. Transcoding of 1 or 2 bytes
;; should happen but eventually `realloc` will fail-fast before any real
;; transcoding. Memory growth is assumed to be VM-based and thus quite fast.
(component definition $A
  (component $A
    (core module $m
      (memory (export "m") 1)
      (global $allow (mut i32) (i32.const 0))
      (func (export "f") (param i32 i32) unreachable)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (local $target-pages i32)
        ;; if realloc isn't allowed, then trap
        global.get $allow
        i32.eqz
        if unreachable end

        i32.const 0
        global.set $allow

        (local.set $target-pages
          (i32.shr_u
            (i32.add
              (local.get 3)
              (i32.const 65535))
            (i32.const 16)))

        (if (i32.lt_u (memory.size) (local.get $target-pages))
          (then
            (memory.grow (i32.sub (local.get $target-pages) (memory.size)))
            i32.const -1
            i32.eq
            if unreachable end
          )
        )

        i32.const 0
      )

      (func (export "allow-one-realloc")
        (global.set $allow (i32.const 1)))
    )
    (core instance $i (instantiate $m))
    (func (export "utf8") (param "x" string)
      (canon lift
        (core func $i "f")
        (memory $i "m")
        (realloc (func $i "realloc"))
        string-encoding=utf8
      )
    )
    (func (export "utf16") (param "x" string)
      (canon lift
        (core func $i "f")
        (memory $i "m")
        (realloc (func $i "realloc"))
        string-encoding=utf16
      )
    )
    (func (export "latin1-utf16") (param "x" string)
      (canon lift
        (core func $i "f")
        (memory $i "m")
        (realloc (func $i "realloc"))
        string-encoding=latin1+utf16
      )
    )

    (func (export "allow-one-realloc") (canon lift (core func $i "allow-one-realloc")))
  )
  (instance $a (instantiate $A))

  (component $B
    (import "a" (instance $a
      (export "utf8" (func (param "x" string)))
      (export "utf16" (func (param "x" string)))
      (export "latin1-utf16" (func (param "x" string)))
    ))

    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $utf8-utf8 (canon lower (func $a "utf8") string-encoding=utf8 (memory $libc "mem")))
    (core func $utf8-utf16 (canon lower (func $a "utf16") string-encoding=utf8 (memory $libc "mem")))
    (core func $utf8-latin1+utf16 (canon lower (func $a "latin1-utf16") string-encoding=utf8 (memory $libc "mem")))

    (core func $utf16-utf8 (canon lower (func $a "utf8") string-encoding=utf16 (memory $libc "mem")))
    (core func $utf16-utf16 (canon lower (func $a "utf16") string-encoding=utf16 (memory $libc "mem")))
    (core func $utf16-latin1+utf16 (canon lower (func $a "latin1-utf16") string-encoding=utf16 (memory $libc "mem")))

    (core func $latin1+utf16-utf8 (canon lower (func $a "utf8") string-encoding=latin1+utf16 (memory $libc "mem")))
    (core func $latin1+utf16-utf16 (canon lower (func $a "utf16") string-encoding=latin1+utf16 (memory $libc "mem")))
    (core func $latin1+utf16-latin1+utf16 (canon lower (func $a "latin1-utf16") string-encoding=latin1+utf16 (memory $libc "mem")))

    (core module $m
      (import "" "utf8-utf8" (func $utf8-utf8 (param i32 i32)))
      (import "" "utf8-utf16" (func $utf8-utf16 (param i32 i32)))
      (import "" "utf8-latin1+utf16" (func $utf8-latin1+utf16 (param i32 i32)))

      (import "" "utf16-utf8" (func $utf16-utf8 (param i32 i32)))
      (import "" "utf16-utf16" (func $utf16-utf16 (param i32 i32)))
      (import "" "utf16-latin1+utf16" (func $utf16-latin1+utf16 (param i32 i32)))

      (import "" "latin1+utf16-utf8" (func $latin1+utf16-utf8 (param i32 i32)))
      (import "" "latin1+utf16-utf16" (func $latin1+utf16-utf16 (param i32 i32)))
      (import "" "latin1+utf16-latin1+utf16" (func $latin1+utf16-latin1+utf16 (param i32 i32)))

      (import "" "mem" (memory 1))

      (func (export "utf8-utf8") (param i32)
        (call $utf8-utf8 (i32.const 0) (local.get 0)))
      (func (export "utf8-utf16") (param i32)
        (call $utf8-utf16 (i32.const 0) (local.get 0)))
      (func (export "utf8-latin1+utf16") (param i32)
        (call $utf8-latin1+utf16 (i32.const 0) (local.get 0)))

      (func (export "utf16-utf8") (param i32)
        (call $utf16-utf8 (i32.const 0) (local.get 0)))
      (func (export "utf16-utf16") (param i32)
        (call $utf16-utf16 (i32.const 0) (local.get 0)))
      (func (export "utf16-latin1+utf16") (param i32)
        (call $utf16-latin1+utf16 (i32.const 0) (local.get 0)))

      (func (export "latin1+utf16-utf8") (param i32)
        (call $latin1+utf16-utf8 (i32.const 0) (local.get 0)))
      (func (export "latin1+utf16-utf16") (param i32)
        (call $latin1+utf16-utf16 (i32.const 0) (local.get 0)))
      (func (export "latin1+utf16-latin1+utf16") (param i32)
        (call $latin1+utf16-latin1+utf16 (i32.const 0) (local.get 0)))

      (func (export "grow") (param i32) (result i32)
        (memory.grow (local.get 0)))
      (func (export "store8") (param i32 i32)
        (i32.store8 (local.get 0) (local.get 1)))
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "utf8-utf8" (func $utf8-utf8))
        (export "utf8-utf16" (func $utf8-utf16))
        (export "utf8-latin1+utf16" (func $utf8-latin1+utf16))
        (export "utf16-utf8" (func $utf16-utf8))
        (export "utf16-utf16" (func $utf16-utf16))
        (export "utf16-latin1+utf16" (func $utf16-latin1+utf16))
        (export "latin1+utf16-utf8" (func $latin1+utf16-utf8))
        (export "latin1+utf16-utf16" (func $latin1+utf16-utf16))
        (export "latin1+utf16-latin1+utf16" (func $latin1+utf16-latin1+utf16))

        (export "mem" (memory $libc "mem"))
      ))
    ))

    (func (export "utf8-utf8") (param "x" u32) (canon lift (core func $i "utf8-utf8")))
    (func (export "utf8-utf16") (param "x" u32) (canon lift (core func $i "utf8-utf16")))
    (func (export "utf8-latin1-utf16") (param "x" u32) (canon lift (core func $i "utf8-latin1+utf16")))

    (func (export "utf16-utf8") (param "x" u32) (canon lift (core func $i "utf16-utf8")))
    (func (export "utf16-utf16") (param "x" u32) (canon lift (core func $i "utf16-utf16")))
    (func (export "utf16-latin1-utf16") (param "x" u32) (canon lift (core func $i "utf16-latin1+utf16")))

    (func (export "latin1-utf16-utf8") (param "x" u32) (canon lift (core func $i "latin1+utf16-utf8")))
    (func (export "latin1-utf16-utf16") (param "x" u32) (canon lift (core func $i "latin1+utf16-utf16")))
    (func (export "latin1-utf16-latin1-utf16") (param "x" u32) (canon lift (core func $i "latin1+utf16-latin1+utf16")))

    (func (export "grow") (param "x" u32) (result s32)
      (canon lift (core func $i "grow")))
    (func (export "store8") (param "addr" u32) (param "val" u8)
      (canon lift (core func $i "store8")))
  )
  (instance $b (instantiate $B (with "a" (instance $a))))

  (export "utf8-utf8" (func $b "utf8-utf8"))
  (export "utf8-utf16" (func $b "utf8-utf16"))
  (export "utf8-latin1-utf16" (func $b "utf8-latin1-utf16"))

  (export "utf16-utf8" (func $b "utf16-utf8"))
  (export "utf16-utf16" (func $b "utf16-utf16"))
  (export "utf16-latin1-utf16" (func $b "utf16-latin1-utf16"))

  (export "latin1-utf16-utf8" (func $b "latin1-utf16-utf8"))
  (export "latin1-utf16-utf16" (func $b "latin1-utf16-utf16"))
  (export "latin1-utf16-latin1-utf16" (func $b "latin1-utf16-latin1-utf16"))

  (export "grow" (func $b "grow"))
  (export "store8" (func $b "store8"))
  (export "allow-one-realloc" (func $a "allow-one-realloc"))
)


;; Test the 9 various permutations below. For each permutation there's a test
;; for the string flat-out being out-of-bounds, a test for a just in-bounds
;; string which hits `unreachable` in the realloc, and then a test for a just
;; out-of-bounds string.

;; utf8 -> utf8 -- can pass up to (1<<31)-1
(component instance $A $A)
(assert_trap (invoke "utf8-utf8" (u32.const 0x7fff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf8-utf8" (u32.const 0x7fff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf8-utf8" (u32.const 0x8000_0000)) "string content out-of-bounds")

;; utf8 -> utf16 -- worst case alloc up-front means that the maximum byte length
;; is half the prior case.
(component instance $A $A)
(assert_trap (invoke "utf8-utf16" (u32.const 0x3fff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf8-utf16" (u32.const 0x3fff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf8-utf16" (u32.const 0x4000_0000)) "string content out-of-bounds")

;; utf8 -> latin1+utf16 -- initial utf8-string can't be too big
(component instance $A $A)
(assert_trap (invoke "utf8-latin1-utf16" (u32.const 0x7fff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf8-latin1-utf16" (u32.const 0x7fff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf8-latin1-utf16" (u32.const 0x8000_0000)) "string content out-of-bounds")

;; utf8 -> latin1+utf16 -- mid-transcode inflation to utf16 has limits on size
;; which only shows up on the second realloc.
;;
;; here `chr(0x100).encode('utf8') == "\xc4\x80"
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_return (invoke "allow-one-realloc"))
(assert_return (invoke "store8" (u32.const 0) (u8.const 0xc4)))
(assert_return (invoke "store8" (u32.const 1) (u8.const 0x80)))
(assert_trap (invoke "utf8-latin1-utf16" (u32.const 0x3fff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_return (invoke "allow-one-realloc"))
(assert_return (invoke "store8" (u32.const 0) (u8.const 0xc4)))
(assert_return (invoke "store8" (u32.const 1) (u8.const 0x80)))
(assert_trap (invoke "utf8-latin1-utf16" (u32.const 0x4000_0000)) "string content out-of-bounds")

;; utf16 -> utf8 -- if all utf16 code units become 1 utf16 byte then up to
;; (1<<30)-1 utf16 codepoints are allowed
(component instance $A $A)
(assert_trap (invoke "utf16-utf8" (u32.const 0x3fff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf16-utf8" (u32.const 0x3fff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf16-utf8" (u32.const 0x4000_0000)) "string content out-of-bounds")

;; utf16 -> utf8 -- if a utf16 code unit becomes more than one utf8 byte then
;; ((1<<31)-1)/3 utf16 codepoints are allowed
;;
;; "ÿ" in utf16 is two bytes, "\xff\x00", and encodes as a multi-byte value in
;; utf8 which causes encoding to switch inflate the utf-8 buffer to the maximum
;; length. This means that a single realloc happens, a few bytes are transcoded,
;; and then a second realloc happens. The boundary around invoking this second
;; realloc and testing for too-large a string is what's tested here.
(component instance $A $A)
(assert_trap (invoke "utf16-utf8" (u32.const 715827882)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_return (invoke "allow-one-realloc"))
(assert_return (invoke "store8" (u32.const 0) (u8.const 0xff)))
(assert_return (invoke "store8" (u32.const 1) (u8.const 0x00)))
(assert_trap (invoke "utf16-utf8" (u32.const 715827882)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_return (invoke "allow-one-realloc"))
(assert_return (invoke "store8" (u32.const 0) (u8.const 0xff)))
(assert_return (invoke "store8" (u32.const 1) (u8.const 0x00)))
(assert_trap (invoke "utf16-utf8" (u32.const 715827883)) "string content out-of-bounds")

;; utf16 -> utf16 -- (1<<30)-1 utf16 codepoints are allowed
(component instance $A $A)
(assert_trap (invoke "utf16-utf16" (u32.const 0x3fff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf16-utf16" (u32.const 0x3fff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf16-utf16" (u32.const 0x4000_0000)) "string content out-of-bounds")

;; utf16 -> latin1+utf16 -- initial utf16-string can't be too big
(component instance $A $A)
(assert_trap (invoke "utf16-latin1-utf16" (u32.const 0x3fff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf16-latin1-utf16" (u32.const 0x3fff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "utf16-latin1-utf16" (u32.const 0x4000_0000)) "string content out-of-bounds")

;; utf16 -> latin1+utf16 -- mid-transcode inflation to utf16 has limits on size
;; which only shows up on the second realloc.
;;
;; here `chr(0x100).encode('utf-16-le') == "\x00\x01"
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_return (invoke "allow-one-realloc"))
(assert_return (invoke "store8" (u32.const 0) (u8.const 0x00)))
(assert_return (invoke "store8" (u32.const 1) (u8.const 0x01)))
(assert_trap (invoke "utf16-latin1-utf16" (u32.const 0x3fff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_return (invoke "allow-one-realloc"))
(assert_return (invoke "store8" (u32.const 0) (u8.const 0x00)))
(assert_return (invoke "store8" (u32.const 1) (u8.const 0x01)))
(assert_trap (invoke "utf16-latin1-utf16" (u32.const 0x4000_0000)) "string content out-of-bounds")

;; latin1+utf16 -> utf8 / latin1 -> utf8 - if it's all single-byte utf8
;; characters there's no actual limit.
(component instance $A $A)
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0x3fff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0x3fff_ffff)) "unreachable")

;; latin1+utf16 -> utf8 / latin1 -> utf8 - with a multi-byte character there's
;; a size limit.
(component instance $A $A)
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0x3fff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_return (invoke "allow-one-realloc"))
(assert_return (invoke "store8" (u32.const 0) (u8.const 0xff)))
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0x3fff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_return (invoke "allow-one-realloc"))
(assert_return (invoke "store8" (u32.const 0) (u8.const 0xff)))
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0x4000_0000)) "string content out-of-bounds")

;; latin1+utf16 -> utf8 / utf16 -> utf8 - if it's all single-byte utf8
;; characters there's no actual limit.
(component instance $A $A)
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0x3fff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0x3fff_ffff)) "unreachable")

;; latin1+utf16 -> utf8 / utf16 -> utf8 - string can be too large just like
;; normal utf16 -> utf8 path.
(component instance $A $A)
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0xbfff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0xbfff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0xc000_0000)) "string content out-of-bounds")

;; latin1+utf16 -> utf8 / utf16 -> utf8 - with multi-byte characters we're
;; limited unlike the
;;
;; for more details see the utf16 -> utf8 case far above. Note that
;; `715827882 == 0x2aaaaaaa` and with the upper bit set that's 0xaaaaaaaa
(component instance $A $A)
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0xaaaaaaaa)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_return (invoke "allow-one-realloc"))
(assert_return (invoke "store8" (u32.const 0) (u8.const 0xff)))
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0xaaaaaaaa)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_return (invoke "allow-one-realloc"))
(assert_return (invoke "store8" (u32.const 0) (u8.const 0xff)))
(assert_trap (invoke "latin1-utf16-utf8" (u32.const 0xaaaaaaab)) "string content out-of-bounds")

;; latin1+utf16 -> utf16 / latin1 -> utf16 - simple inflation but the string
;; can be too big since it's doubling in byte size.
(component instance $A $A)
(assert_trap (invoke "latin1-utf16-utf16" (u32.const 0x3fff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "latin1-utf16-utf16" (u32.const 0x3fff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "latin1-utf16-utf16" (u32.const 0x4000_0000)) "string content out-of-bounds")

;; latin1+utf16 -> utf16 / utf16 -> utf16 - simple inflation, but string can be
;; too large.
(component instance $A $A)
(assert_trap (invoke "latin1-utf16-utf16" (u32.const 0xbfff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "latin1-utf16-utf16" (u32.const 0xbfff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "latin1-utf16-utf16" (u32.const 0xc000_0000)) "string content out-of-bounds")

;; latin1+utf16 -> latin1+utf16 - latin1 src is a simple copy with no limit.
(component instance $A $A)
(assert_trap (invoke "latin1-utf16-latin1-utf16" (u32.const 0x7fff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "latin1-utf16-latin1-utf16" (u32.const 0x7fff_ffff)) "unreachable")

;; latin1+utf16 -> latin1+utf16 - utf16 src means that the string can be
;; too large, so test that here.
(component instance $A $A)
(assert_trap (invoke "latin1-utf16-latin1-utf16" (u32.const 0xbfff_ffff)) "string content out-of-bounds")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "latin1-utf16-latin1-utf16" (u32.const 0xbfff_ffff)) "unreachable")
(component instance $A $A)
(assert_return (invoke "grow" (u32.const 65530)) (s32.const 1))
(assert_trap (invoke "latin1-utf16-latin1-utf16" (u32.const 0xc000_0000)) "string content out-of-bounds")
