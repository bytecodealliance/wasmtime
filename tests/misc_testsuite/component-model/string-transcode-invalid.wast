;;! multi_memory = true

;; Transcode a utf16 string to latin1+utf16, but the original string is
;; out-of-bounds. Should report a first-class error.
(component
  (component $dst
    (core module $m
      (func (export "recv") (param i32 i32))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
      (memory (export "memory") 1)
    )
    (core instance $i (instantiate $m))
    (func (export "recv") (param "a" string)
      (canon lift (core func $i "recv") (realloc (func $i "realloc")) (memory $i "memory")
        string-encoding=latin1+utf16)
    )
  )

  ;; Source component: uses utf16 encoding.
  ;; Passes a string placed near the END of linear memory to $dst.
  (component $src
    (import "recv" (func $recv (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)  ;; 1 page = 65536 bytes
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 0)
    )
    (core instance $libc (instantiate $libc))
    ;; canon lower with utf16 — the source side of the mismatch.
    (core func $recv_lowered (canon lower (func $recv) string-encoding=utf16 (memory $libc "memory")))
    (core module $m
      (import "" "" (func $recv (param i32 i32)))
      (import "libc" "memory" (memory 0))
      (func (export "run")
        ;; Write 8 UTF-16 code units (16 bytes) of 'A' (U+0041) at the end of memory.
        ;; Offsets 65520–65535: exactly fills to the last byte of the page.
        (i32.store (i32.const 65520) (i32.const 0x00410041))
        (i32.store (i32.const 65524) (i32.const 0x00410041))
        (i32.store (i32.const 65528) (i32.const 0x00410041))
        (i32.store (i32.const 65532) (i32.const 0x00410041))

        ;; Pass ptr=65520, len=10 CODE UNITS (not bytes).
        ;; We wrote 8 code units but claim 10 — the extra 2 are past end of memory.
        (call $recv (i32.const 65520) (i32.const 10))
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "" (func $recv_lowered))))
      (with "libc" (instance $libc))
    ))
    (func (export "run")
      (canon lift (core func $i "run"))
    )
  )

  ;; Wire the components together and run.
  (instance $dst_inst (instantiate $dst))
  (instance $i (instantiate $src (with "recv" (func $dst_inst "recv"))))

  (export "run" (func $i "run"))
)

(assert_trap (invoke "run") "string content out-of-bounds")
