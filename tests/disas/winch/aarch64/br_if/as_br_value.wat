;;! target = "aarch64"
;;! test = "winch"
(module
  (func (export "as-br-value") (result i32)
    (block (result i32) (br 0 (br_if 0 (i32.const 1) (i32.const 2))))
  )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x10
;;       mov     x28, sp
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x16, #2
;;       mov     w1, w16
;;       mov     x16, #1
;;       mov     w0, w16
;;       tst     w1, w1
;;       b.ne    #0x3c
;;       b       #0x3c
;;   3c: add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
