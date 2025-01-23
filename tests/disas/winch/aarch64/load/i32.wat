;;! target = "aarch64"
;;! test = "winch"
(module
  (memory 1)
  (func (export "as-br-value") (result i32)
    (block (result i32) (br 0 (i32.load (i32.const 0))))
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
;;       mov     x16, #0
;;       mov     w0, w16
;;       ldur    x1, [x9, #0x50]
;;       add     x1, x1, x0, uxtx
;;       ldur    w0, [x1]
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
