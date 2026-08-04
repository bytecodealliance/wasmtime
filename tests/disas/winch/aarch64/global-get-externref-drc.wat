;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-Ccollector=drc"

(module
  (global $g (mut externref) (ref.null extern))
  (func (export "get") (result externref)
    (global.get $g)))
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x20
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x148
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       ldur    w0, [x9, #0x30]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       ldur    w0, [x28]
;;       tst     w0, w0
;;       b.eq    #0x124
;;       b       #0x60
;;   60: mov     w1, w0
;;       and     w1, w1, #1
;;       tst     w1, w1
;;       b.ne    #0x124
;;       b       #0x74
;;   74: ldur    x1, [x9, #8]
;;       ldur    x2, [x1, #0x28]
;;       ldur    x1, [x1, #0x20]
;;       mov     x3, x0
;;       add     x3, x3, #0x14
;;       cmp     x3, x2, uxtx
;;       sub     sp, x28, #0xc
;;       b.hi    #0x14c
;;   94: mov     sp, x28
;;       mov     x2, x1
;;       add     x2, x2, x0, uxtx
;;       ldur    w3, [x2]
;;       and     w3, w3, #2
;;       tst     w3, w3
;;       b.ne    #0x124
;;       b       #0xb4
;;   b4: ldur    x4, [x9, #0x20]
;;       ldur    w3, [x4]
;;       stur    w3, [x2, #0x10]
;;       ldur    w3, [x2]
;;       orr     w3, w3, #2
;;       stur    w3, [x2]
;;       ldur    x3, [x2, #8]
;;       add     x3, x3, #1
;;       stur    x3, [x2, #8]
;;       stur    w0, [x4]
;;       ldur    w3, [x4, #4]
;;       add     w3, w3, #1
;;       stur    w3, [x4, #4]
;;       ldur    w2, [x4, #8]
;;       add     w2, w2, w2, uxtx
;;       cmp     w3, w2, uxtx
;;       b.lo    #0x124
;;       b       #0xfc
;;   fc: cmp     w3, #0x400
;;       b.lo    #0x124
;;       b       #0x108
;;  108: sub     x28, x28, #0xc
;;       mov     sp, x28
;;       mov     x0, x9
;;       bl      #0x29c
;;  118: add     x28, x28, #0xc
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[12]
;;       mov     sp, x28
;;       ldur    x9, [x28, #0xc]
;;       ldur    w0, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  148: udf     #0xc11f
;;  14c: udf     #0xc11f
