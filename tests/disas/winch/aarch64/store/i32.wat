;;! target = "aarch64"
;;! test = "winch"
(module
  (memory 1)

  (func (export "as-block-value")
    (block (i32.store (i32.const 0) (i32.const 1)))
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
;;       mov     x16, #1
;;       mov     w0, w16
;;       mov     x16, #0
;;       mov     w1, w16
;;       ldur    x2, [x9, #0x50]
;;       add     x2, x2, x1, uxtx
;;       stur    w0, [x2]
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
