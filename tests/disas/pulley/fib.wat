;;! target = "pulley64"
;;! test = "compile"

;; Test of a recursive fibonacci routine and its codegen
;;
;; FIXME(#9942) this test currently has an extraneous `xmov` after the second
;; call instruction.

(module
  (func $fib (export "fib") (param $n i32) (result i32)
    (if (result i32)
      (i32.eq
        (i32.const 0)
        (local.get $n)
      )
      (then
        (i32.const 1)
      )
      (else
        (if (result i32)
          (i32.eq
            (i32.const 1)
            (local.get $n)
          )
          (then
            (i32.const 1)
          )
          (else
            (i32.add
              ;; fib(n - 1)
              (call $fib
                (i32.add
                  (local.get $n)
                  (i32.const -1)
                )
              )
              ;; fib(n - 2)
              (call $fib
                (i32.add
                  (local.get $n)
                  (i32.const -2)
                )
              )
            )
          )
        )
      )
    )
  )
)
;; wasm[0]::function[0]::fib:
;;       push_frame_save 32, x17, x24, x29
;;       br_if_xeq32_i8 x2, 0, 0x47    // target = 0x4c
;;       br_if_xeq32_i8 x2, 1, 0x39    // target = 0x45
;;   13: xsub32_u8 x14, x2, 1
;;       xmov x24, x0
;;       xmov x29, x2
;;       call3 x24, x24, x14, -0x1d    // target = 0x0
;;       xmov x17, x0
;;       xmov x2, x29
;;       xmov x0, x24
;;       xsub32_u8 x14, x2, 2
;;       call3 x0, x0, x14, -0x32    // target = 0x0
;;       xmov x5, x17
;;       xadd32 x0, x5, x0
;;       jump 0xe    // target = 0x4e
;;   45: xone x0
;;       jump 0x7    // target = 0x4e
;;   4c: xone x0
;;       pop_frame_restore 32, x17, x24, x29
;;       ret
