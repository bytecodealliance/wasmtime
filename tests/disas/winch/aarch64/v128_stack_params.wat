;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func $take (param v128) (param v128) (param v128) (param v128) (param v128) (param v128) (param v128) (param v128) (param v128) (param v128) (result i32)
    (v128.store (i32.const 0) (local.get 9))
    (i32.load (i32.const 0)))
  (func (export "drive") (result i32)
    (call $take
      (v128.const i64x2 0 0)
      (v128.const i64x2 1 1)
      (v128.const i64x2 2 2)
      (v128.const i64x2 3 3)
      (v128.const i64x2 4 4)
      (v128.const i64x2 5 5)
      (v128.const i64x2 6 6)
      (v128.const i64x2 7 7)
      (v128.const i64x2 8 8)
      (v128.const i64x2 9 9))))
;; wasm[0]::function[0]::take:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x90
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0xa0
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x90
;;       mov     sp, x28
;;       stur    x0, [x28, #0x88]
;;       stur    x1, [x28, #0x80]
;;       stur    q0, [x28, #0x70]
;;       stur    q1, [x28, #0x60]
;;       stur    q2, [x28, #0x50]
;;       stur    q3, [x28, #0x40]
;;       stur    q4, [x28, #0x30]
;;       stur    q5, [x28, #0x20]
;;       stur    q6, [x28, #0x10]
;;       stur    q7, [x28]
;;       ldur    q0, [x29, #0x20]
;;       mov     x0, #0
;;       ldur    x1, [x9, #0x38]
;;       add     x1, x1, w0, uxtw
;;       stur    q0, [x1]
;;       mov     x0, #0
;;       ldur    x1, [x9, #0x38]
;;       add     x1, x1, w0, uxtw
;;       ldur    w0, [x1]
;;       add     x28, x28, #0x90
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       add     sp, sp, #0x20
;;       ret
;;   a0: udf     #0xc11f
;;
;; wasm[0]::function[1]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x30
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x164
;;   ec: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, x9
;;       ldr     q0, #0x170
;;       ldr     q1, #0x180
;;       ldr     q2, #0x190
;;       ldr     q3, #0x1a0
;;       ldr     q4, #0x1b0
;;       ldr     q5, #0x1c0
;;       ldr     q6, #0x1d0
;;       ldr     q7, #0x1e0
;;       ldr     q31, #0x1f0
;;       stur    q31, [x28]
;;       ldr     q31, #0x200
;;       stur    q31, [x28, #0x10]
;;       bl      #0
;;  144: mov     x28, sp
;;       ldur    x9, [x28, #8]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  164: udf     #0xc11f
;;  168: udf     #0
;;  16c: udf     #0
;;  170: udf     #0
;;  174: udf     #0
;;  178: udf     #0
;;  17c: udf     #0
;;  180: udf     #1
;;  184: udf     #0
;;  188: udf     #1
;;  18c: udf     #0
;;  190: udf     #2
;;  194: udf     #0
;;  198: udf     #2
;;  19c: udf     #0
;;  1a0: udf     #3
;;  1a4: udf     #0
;;  1a8: udf     #3
;;  1ac: udf     #0
;;  1b0: udf     #4
;;  1b4: udf     #0
;;  1b8: udf     #4
;;  1bc: udf     #0
;;  1c0: udf     #5
;;  1c4: udf     #0
;;  1c8: udf     #5
;;  1cc: udf     #0
;;  1d0: udf     #6
;;  1d4: udf     #0
;;  1d8: udf     #6
;;  1dc: udf     #0
;;  1e0: udf     #7
;;  1e4: udf     #0
;;  1e8: udf     #7
;;  1ec: udf     #0
;;  1f0: udf     #8
;;  1f4: udf     #0
;;  1f8: udf     #8
;;  1fc: udf     #0
;;  200: udf     #9
;;  204: udf     #0
;;  208: udf     #9
;;  20c: udf     #0
