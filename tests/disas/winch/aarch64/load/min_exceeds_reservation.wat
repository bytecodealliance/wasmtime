;;! target = "aarch64"
;;! test = "winch"
;;! flags = ["-Omemory-reservation=0", "-Omemory-may-move=n"]

;; The memory's minimum size exceeds `memory-reservation`, so the runtime
;; sizes the allocation from the minimum and the reservation is not an upper
;; bound: this access must get a dynamic bounds check, not a static trap.
(module
  (memory 2)
  (func (export "load_offset") (param i32) (result i32)
    local.get 0
    i32.load offset=70000))
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x18
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0xbc
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       ldur    w0, [x28, #4]
;;       ldur    x1, [x9, #0x40]
;;       mov     w2, w0
;;       sub     sp, x28, #8
;;       mov     w16, #0x1174
;;       movk    w16, #1, lsl #16
;;       adds    x2, x2, x16, uxtx
;;       b.hs    #0xc0
;;   64: mov     sp, x28
;;       cmp     x2, x1, uxtx
;;       sub     sp, x28, #8
;;       b.hi    #0xc4
;;   74: mov     sp, x28
;;       ldur    x3, [x9, #0x38]
;;       add     x3, x3, w0, uxtw
;;       mov     w16, #0x1170
;;       movk    w16, #1, lsl #16
;;       add     x3, x3, x16, uxtx
;;       mov     x4, #0
;;       cmp     x2, x1, uxtx
;;       csel    x3, x4, x3, hi
;;       sub     sp, x28, #8
;;       ldur    w0, [x3]
;;       mov     sp, x28
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   bc: udf     #0xc11f
;;   c0: udf     #0xc11f
;;   c4: udf     #0xc11f
