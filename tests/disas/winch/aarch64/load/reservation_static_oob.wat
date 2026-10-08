;;! target = "aarch64"
;;! test = "winch"
;;! flags = ["-Omemory-reservation=0x20000", "-Omemory-may-move=n"]

;; The memory's minimum size fits within `memory-reservation` and the memory
;; can't move, so the reservation is an upper bound: an access whose offset
;; exceeds it is out of bounds and must trap, via the regular bounds check.
(module
  (memory 2)
  (func (export "load_offset") (param i32) (result i32)
    local.get 0
    i32.load offset=0x20000))
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
;;       b.lo    #0xb4
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
;;       mov     w16, #4
;;       movk    w16, #2, lsl #16
;;       adds    x2, x2, x16, uxtx
;;       b.hs    #0xb8
;;   64: mov     sp, x28
;;       cmp     x2, x1, uxtx
;;       sub     sp, x28, #8
;;       b.hi    #0xbc
;;   74: mov     sp, x28
;;       ldur    x3, [x9, #0x38]
;;       add     x3, x3, w0, uxtw
;;       add     x3, x3, #0x20, lsl #12
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
;;   b4: udf     #0xc11f
;;   b8: udf     #0xc11f
;;   bc: udf     #0xc11f
