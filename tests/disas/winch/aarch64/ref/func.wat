;;! target = "aarch64"
;;! test = "winch"

;; `ref.func` of a defined function initializes its `VMFuncRef` within the
;; `vmctx` inline, using relocations for the function's code pointers.

(module
  (elem declare func $f)
  (func $f (result i32) i32.const 42)
  (func (export "ref-func") (result funcref)
    (ref.func $f)))
;; wasm[0]::function[0]::f:
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
;;       b.lo    #0x5c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x0, #0x2a
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   5c: udf     #0xc11f
;;
;; wasm[0]::function[1]:
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
;;       b.lo    #0xe4
;;   8c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       adrp    x16, #0
;;       add     x16, x16, #0xe8
;;       stur    x16, [x9, #0x50]
;;       adrp    x16, #0
;;       add     x16, x16, #0
;;       stur    x16, [x9, #0x58]
;;       stur    x9, [x9, #0x68]
;;       ldur    x16, [x9, #0x28]
;;       ldur    w16, [x16]
;;       stur    w16, [x9, #0x60]
;;       add     x0, x9, #0x50
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   e4: udf     #0xc11f
