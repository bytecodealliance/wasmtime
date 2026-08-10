;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-Ccollector=drc"

(module
  (global $g (mut externref) (ref.null extern))
  (func (param externref)
    (global.set $g (local.get 0))))
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
;;       b.lo    #0x168
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       ldur    w16, [x28, #4]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x28]
;;       ldur    w0, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x1, [x9, #8]
;;       ldur    x2, [x1, #0x28]
;;       ldur    x1, [x1, #0x20]
;;       ldur    w3, [x9, #0x30]
;;       tst     w0, w0
;;       b.eq    #0xbc
;;       b       #0x7c
;;   7c: mov     w16, w0
;;       and     w16, w16, #1
;;       tst     w16, w16
;;       b.ne    #0xbc
;;       b       #0x90
;;   90: mov     x16, x0
;;       add     x16, x16, #0x10
;;       cmp     x16, x2, uxtx
;;       sub     sp, x28, #8
;;       b.hi    #0x16c
;;   a4: mov     sp, x28
;;       mov     x4, x1
;;       add     x4, x4, x0, uxtx
;;       ldur    x5, [x4, #8]
;;       add     x5, x5, #1
;;       stur    x5, [x4, #8]
;;       stur    w0, [x9, #0x30]
;;       tst     w3, w3
;;       b.eq    #0x150
;;       b       #0xcc
;;   cc: mov     w16, w3
;;       and     w16, w16, #1
;;       tst     w16, w16
;;       b.ne    #0x150
;;       b       #0xe0
;;   e0: mov     x16, x3
;;       add     x16, x16, #0x10
;;       cmp     x16, x2, uxtx
;;       sub     sp, x28, #8
;;       b.hi    #0x170
;;   f4: mov     sp, x28
;;       mov     x4, x1
;;       add     x4, x4, x3, uxtx
;;       ldur    x5, [x4, #8]
;;       sub     x5, x5, #1
;;       cmp     x5, #0
;;       b.eq    #0x11c
;;       b       #0x114
;;  114: stur    x5, [x4, #8]
;;       b       #0x150
;;  11c: sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w3, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x0, x9
;;       ldur    w1, [x28, #4]
;;       bl      #0x1ec
;;  13c: add     x28, x28, #4
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[12]
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x10]
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  168: udf     #0xc11f
;;  16c: udf     #0xc11f
;;  170: udf     #0xc11f
