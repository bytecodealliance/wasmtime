;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-W tail-call=y"
;;! filter = "tail_caller"

;; The caller has stack arguments, but the tail callee has none. Restore the
;; saved FP before advancing SP past the old frame (issue #14503).
(module
  (func $leaf (param i32) (result i32) local.get 0)
  (func $tail_caller (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
      (result i32)
    local.get 0
    return_call $leaf)

  ;; The entry-SP offset exceeds the 12-bit add-immediate range.
  (func $tail_caller_large (param
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
  ) (result i32)
    i32.const 42
    return_call $leaf))
;; wasm[0]::function[1]::tail_caller:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x2c
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x128
;;   ac: mov     x9, x0
;;       sub     x28, x28, #0x28
;;       mov     sp, x28
;;       stur    x0, [x28, #0x20]
;;       stur    x1, [x28, #0x18]
;;       stur    w2, [x28, #0x14]
;;       stur    w3, [x28, #0x10]
;;       stur    w4, [x28, #0xc]
;;       stur    w5, [x28, #8]
;;       stur    w6, [x28, #4]
;;       stur    w7, [x28]
;;       ldur    w16, [x28, #0x14]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x28]
;;       mov     x0, x9
;;       mov     x1, x9
;;       ldur    w2, [x28]
;;       ldur    x28, [x29, #-0x10]
;;       ldur    x30, [x29, #8]
;;       add     x16, x29, #0x40
;;       ldur    x29, [x29]
;;       mov     sp, x16
;;       b       #0
;;  10c: add     x28, x28, #0x28
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       add     sp, sp, #0x30
;;       ret
;;  128: udf     #0xc11f
;;
;; wasm[0]::function[2]::tail_caller_large:
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
;;       b.lo    #0x1e0
;;  16c: mov     x9, x0
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
;;       mov     x0, x9
;;       mov     x1, x9
;;       mov     x2, #0x2a
;;       ldur    x28, [x29, #-0x10]
;;       ldur    x30, [x29, #8]
;;       mov     x16, #0x1060
;;       add     x16, x16, x29, uxtx
;;       ldur    x29, [x29]
;;       mov     sp, x16
;;       b       #0
;;  1c0: add     x28, x28, #0x40
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       mov     x16, #0x1050
;;       add     sp, sp, x16
;;       ret
;;  1e0: udf     #0xc11f
