;;! target = "aarch64"
;;! test = "winch"

;; Nonconstant arguments force spills. Keep a value live below the arguments,
;; then consume both register results and a separate stack-result area.
(module
  (type $single (func (param i64 i64 i64 i64 i64 i64 i64 i64 i64 i64) (result i64)))
  (func $single (type $single) local.get 0 local.get 9 i64.add)
  (func $multi (param i64 i64 i64 i64 i64 i64 i64 i64 i64 i64) (result i64 i64 f64)
    local.get 0 local.get 9 f64.const 3.5)
  (table funcref (elem $single))
  (func (export "direct") (param i64) (result i64)
    local.get 0
    local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0
    call $single i64.add)
  (func (export "indirect") (param i64) (result i64)
    local.get 0
    local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0
    i32.const 0 call_indirect (type $single) i64.add)
  (func (export "stack_results") (param i64) (result i64)
    local.get 0
    local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0 local.get 0
    call $multi i64.trunc_f64_s i64.add i64.add i64.add))
;; wasm[0]::function[0]::single:
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
;;       b.lo    #0x84
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x40
;;       mov     sp, x28
;;       stur    x0, [x28, #0x38]
;;       stur    x1, [x28, #0x30]
;;       stur    x2, [x28, #0x28]
;;       stur    x3, [x28, #0x20]
;;       stur    x4, [x28, #0x18]
;;       stur    x5, [x28, #0x10]
;;       stur    x6, [x28, #8]
;;       stur    x7, [x28]
;;       ldur    x0, [x29, #0x28]
;;       ldur    x1, [x28, #0x28]
;;       add     x1, x1, x0, uxtx
;;       mov     x0, x1
;;       add     x28, x28, #0x40
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       add     sp, sp, #0x20
;;       ret
;;   84: udf     #0xc11f
;;
;; wasm[0]::function[1]::multi:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x1, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x50
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x15c
;;   cc: mov     x9, x1
;;       sub     x28, x28, #0x40
;;       mov     sp, x28
;;       stur    x1, [x28, #0x38]
;;       stur    x2, [x28, #0x30]
;;       stur    x3, [x28, #0x28]
;;       stur    x4, [x28, #0x20]
;;       stur    x5, [x28, #0x18]
;;       stur    x6, [x28, #0x10]
;;       stur    x7, [x28, #8]
;;       stur    x0, [x28]
;;       fmov    d0, #3.50000000
;;       ldur    x16, [x28, #0x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x29, #0x30]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x0, [x28, #0x10]
;;       ldur    x16, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x0]
;;       ldur    x16, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x0, #8]
;;       add     x28, x28, #0x40
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       add     sp, sp, #0x30
;;       ret
;;  15c: udf     #0xc11f
;;
;; wasm[0]::function[2]:
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
;;       b.lo    #0x2dc
;;  18c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    x2, [x28]
;;       ldur    x16, [x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #8]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x10]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x18]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x20]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x30]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x38]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x40]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x48]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x50]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, x9
;;       ldur    x2, [x28, #0x68]
;;       ldur    x3, [x28, #0x60]
;;       ldur    x4, [x28, #0x58]
;;       ldur    x5, [x28, #0x50]
;;       ldur    x6, [x28, #0x48]
;;       ldur    x7, [x28, #0x40]
;;       ldur    x16, [x28, #0x38]
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x30]
;;       stur    x16, [x28, #8]
;;       ldur    x16, [x28, #0x28]
;;       stur    x16, [x28, #0x10]
;;       ldur    x16, [x28, #0x20]
;;       stur    x16, [x28, #0x18]
;;       bl      #0
;;  2a0: mov     x28, sp
;;       add     x28, x28, #0x50
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x18]
;;       ldur    x1, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       add     x1, x1, x0, uxtx
;;       mov     x0, x1
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  2dc: udf     #0xc11f
;;
;; wasm[0]::function[3]:
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
;;       b.lo    #0x510
;;  30c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    x2, [x28]
;;       ldur    x16, [x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #8]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x10]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x18]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x20]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x30]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x38]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x40]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x48]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x50]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       mov     x1, #0
;;       mov     x2, x9
;;       ldur    x3, [x2, #0x38]
;;       cmp     x1, x3, uxtx
;;       b.hs    #0x514
;;  3e8: mov     x16, x1
;;       mov     x17, #8
;;       mul     x16, x16, x17
;;       ldur    x2, [x2, #0x30]
;;       mov     x4, x2
;;       add     x2, x2, x16, uxtx
;;       cmp     x1, x3, uxtx
;;       csel    x2, x4, x2, hs
;;       ldur    x0, [x2]
;;       tst     x0, x0
;;       b.ne    #0x44c
;;       b       #0x418
;;  418: sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       sub     x28, x28, #0xc
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, #0
;;       ldur    w2, [x28, #0xc]
;;       bl      #0xce0
;;  43c: add     x28, x28, #0x10
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x68]
;;       b       #0x450
;;  44c: and     x0, x0, #0xfffffffffffffffe
;;       cbz     x0, #0x518
;;  454: ldur    x16, [x9, #0x28]
;;       ldur    w1, [x16]
;;       ldur    w2, [x0, #0x10]
;;       cmp     w1, w2, uxtx
;;       b.ne    #0x51c
;;  468: sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x0, [x28]
;;       ldur    x8, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       ldur    x11, [x8, #0x18]
;;       ldur    x10, [x8, #8]
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     x0, x11
;;       mov     x1, x9
;;       ldur    x2, [x28, #0x68]
;;       ldur    x3, [x28, #0x60]
;;       ldur    x4, [x28, #0x58]
;;       ldur    x5, [x28, #0x50]
;;       ldur    x6, [x28, #0x48]
;;       ldur    x7, [x28, #0x40]
;;       ldur    x16, [x28, #0x38]
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x30]
;;       stur    x16, [x28, #8]
;;       ldur    x16, [x28, #0x28]
;;       stur    x16, [x28, #0x10]
;;       ldur    x16, [x28, #0x20]
;;       stur    x16, [x28, #0x18]
;;       blr     x10
;;  4d4: mov     x28, sp
;;       add     x28, x28, #0x50
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x18]
;;       ldur    x1, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       add     x1, x1, x0, uxtx
;;       mov     x0, x1
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  510: udf     #0xc11f
;;  514: udf     #0xc11f
;;  518: udf     #0xc11f
;;  51c: udf     #0xc11f
;;
;; wasm[0]::function[4]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0xb0
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x700
;;  54c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    x2, [x28]
;;       ldur    x16, [x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #8]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x10]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x18]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x20]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x30]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x38]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x40]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x48]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x50]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       sub     x28, x28, #0x30
;;       mov     sp, x28
;;       mov     x1, x9
;;       mov     x2, x9
;;       ldur    x3, [x28, #0x88]
;;       ldur    x4, [x28, #0x80]
;;       ldur    x5, [x28, #0x78]
;;       ldur    x6, [x28, #0x70]
;;       ldur    x7, [x28, #0x68]
;;       ldur    x16, [x28, #0x60]
;;       stur    x16, [x28]
;;       ldur    x16, [x28, #0x58]
;;       stur    x16, [x28, #8]
;;       ldur    x16, [x28, #0x50]
;;       stur    x16, [x28, #0x10]
;;       ldur    x16, [x28, #0x48]
;;       stur    x16, [x28, #0x18]
;;       ldur    x16, [x28, #0x40]
;;       stur    x16, [x28, #0x20]
;;       add     x0, x28, #0x30
;;       bl      #0xa0
;;  670: mov     x28, sp
;;       ldur    x16, [x28, #8]
;;       stur    x16, [x28, #0x58]
;;       ldur    x16, [x28]
;;       stur    x16, [x28, #0x50]
;;       add     x28, x28, #0x50
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x28]
;;       fcmp    d0, d0
;;       b.vs    #0x704
;;  698: ldr     d31, #0x710
;;       fcmp    d0, d31
;;       b.le    #0x708
;;  6a4: ldr     d31, #0x718
;;       fcmp    d0, d31
;;       b.ge    #0x70c
;;  6b0: fcvtzs  x0, d0
;;       ldur    x1, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       add     x1, x1, x0, uxtx
;;       ldur    x0, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       add     x0, x0, x1, uxtx
;;       ldur    x1, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       add     x1, x1, x0, uxtx
;;       mov     x0, x1
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  700: udf     #0xc11f
;;  704: udf     #0xc11f
;;  708: udf     #0xc11f
;;  70c: udf     #0xc11f
;;  710: udf     #1
;;  714: .byte   0x00, 0x00, 0xe0, 0xc3
;;  718: udf     #0
;;  71c: .byte   0x00, 0x00, 0xe0, 0x43
