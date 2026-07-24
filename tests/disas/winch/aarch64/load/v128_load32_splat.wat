;;! target = "aarch64"
;;! test = "winch"

(module
  (memory (data "\80\ff\01\ff\00\02\fe\c0\aa\55\00\00\00\00\00\00"))

  (func (export "v128.load32_splat") (result v128) (v128.load32_splat (i32.const 0)))
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x10
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x68
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x0, #0
;;       ldur    x1, [x9, #0x38]
;;       add     x1, x1, w0, uxtw
;;       ld1r    {v0.4s}, [x1]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   68: udf     #0xc11f
