;;! target = "aarch64"
;;! test = "compile"

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
;;       mov     w2, #0x2a
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[1]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       adrp    x6, #0
;;       add     x6, x6, #0x5c
;;       str     x6, [x2, #0x50]
;;       adrp    x7, #0
;;       add     x7, x7, #0
;;       str     x7, [x2, #0x58]
;;       str     x2, [x2, #0x68]
;;       ldr     x9, [x2, #0x28]
;;       ldr     w9, [x9]
;;       str     w9, [x2, #0x60]
;;       add     x2, x2, #0x50
;;       ldp     x29, x30, [sp], #0x10
;;       ret
