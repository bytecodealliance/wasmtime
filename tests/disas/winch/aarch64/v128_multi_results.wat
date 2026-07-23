;;! target = "aarch64"
;;! test = "winch"

(module
  (func $mr (result v128 i32)
    (v128.const i64x2 1 2)
    (i32.const 5))
  (func (export "drive") (result i32) (local $x i32) (local $v v128)
    (call $mr)
    (local.set $x)
    (local.set $v)
    (local.get $x)))
;; wasm[0]::function[0]::mr:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x1, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x28
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x84
;;   2c: mov     x9, x1
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x1, [x28, #0x10]
;;       stur    x2, [x28, #8]
;;       stur    x0, [x28]
;;       mov     x0, #5
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       ldr     q31, #0x90
;;       stur    q31, [x28]
;;       ldur    x1, [x28, #0x10]
;;       ldur    q31, [x28]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    q31, [x1]
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   84: udf     #0xc11f
;;   88: udf     #0
;;   8c: udf     #0
;;   90: udf     #1
;;   94: udf     #0
;;   98: udf     #2
;;   9c: udf     #0
;;
;; wasm[0]::function[1]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x40
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x140
;;   cc: mov     x9, x0
;;       sub     x28, x28, #0x30
;;       mov     sp, x28
;;       stur    x0, [x28, #0x28]
;;       stur    x1, [x28, #0x20]
;;       mov     x16, #0
;;       stur    x16, [x28, #0x18]
;;       stur    x16, [x28, #0x10]
;;       stur    x16, [x28, #8]
;;       stur    x16, [x28]
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     x1, x9
;;       mov     x2, x9
;;       add     x0, x28, #0
;;       bl      #0
;;  10c: ldur    x9, [x28, #0x38]
;;       stur    w0, [x28, #0x2c]
;;       ldur    q0, [x28]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    q0, [x28]
;;       ldur    w0, [x28, #0x1c]
;;       add     x28, x28, #0x30
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  140: udf     #0xc11f
