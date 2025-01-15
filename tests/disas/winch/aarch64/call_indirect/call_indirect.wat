;;! target="aarch64"
;;! test = "winch"

(module
  (type $over-i32 (func (param i32) (result i32)))

  (table funcref
    (elem
      $fib-i32
    )
  )

  (func $fib-i32 (export "fib-i32") (type $over-i32)
    (if (result i32) (i32.le_u (local.get 0) (i32.const 1))
      (then (i32.const 1))
      (else
        (i32.add
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 2))
            (i32.const 0)
          )
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 1))
            (i32.const 0)
          )
        )
      )
    )
  )
)

;; wasm[0]::function[0]::fib-i32:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x18
;;       mov     x28, sp
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       ldur    w0, [x28, #4]
;;       cmp     w0, #1
;;       cset    x0, ls
;;       tst     w0, w0
;;       b.eq    #0x48
;;       b       #0x3c
;;   3c: mov     x16, #1
;;       mov     w0, w16
;;       b       #0x250
;;   48: ldur    w0, [x28, #4]
;;       sub     w0, w0, #2
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w0, [x28]
;;       mov     x16, #0
;;       mov     w1, w16
;;       mov     x2, x9
;;       ldur    x3, [x2, #0x60]
;;       cmp     x1, x3, uxtx
;;       b.hs    #0x260
;;   74: mov     x16, x1
;;       mov     x16, #8
;;       mul     x16, x16, x16
;;       ldur    x2, [x2, #0x58]
;;       mov     x4, x2
;;       add     x2, x2, x16, uxtx
;;       cmp     w1, w3, uxtx
;;       csel    x2, x4, x4, hs
;;       ldur    x0, [x2]
;;       tst     x0, x0
;;       b.ne    #0xd4
;;       b       #0xa4
;;   a4: sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w1, [x28]
;;       mov     x0, x9
;;       mov     x16, #0
;;       mov     w1, w16
;;       ldur    w2, [x28]
;;       bl      #0x398
;;   c4: add     sp, sp, #4
;;       mov     x28, sp
;;       ldur    x9, [x28, #0x14]
;;       b       #0xd8
;;   d4: and     x0, x0, #0xfffffffffffffffe
;;       cbz     x0, #0x264
;;   dc: ldur    x16, [x9, #0x50]
;;       ldur    w1, [x16]
;;       ldur    w2, [x0, #0x10]
;;       cmp     w1, w2, uxtx
;;       b.ne    #0x268
;;   f0: sub     sp, sp, #8
;;       mov     x28, sp
;;       stur    x0, [x28]
;;       ldur    x3, [x28]
;;       add     sp, sp, #8
;;       mov     x28, sp
;;       ldur    x5, [x3, #0x18]
;;       ldur    x4, [x3, #8]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       mov     x0, x5
;;       mov     x1, x9
;;       ldur    w2, [x28, #4]
;;       blr     x4
;;  128: add     sp, sp, #4
;;       mov     x28, sp
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       ldur    x9, [x28, #0x10]
;;       ldur    w1, [x28, #4]
;;       sub     w1, w1, #1
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w0, [x28]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w1, [x28]
;;       mov     x16, #0
;;       mov     w1, w16
;;       mov     x2, x9
;;       ldur    x3, [x2, #0x60]
;;       cmp     x1, x3, uxtx
;;       b.hs    #0x26c
;;  174: mov     x16, x1
;;       mov     x16, #8
;;       mul     x16, x16, x16
;;       ldur    x2, [x2, #0x58]
;;       mov     x4, x2
;;       add     x2, x2, x16, uxtx
;;       cmp     w1, w3, uxtx
;;       csel    x2, x4, x4, hs
;;       ldur    x0, [x2]
;;       tst     x0, x0
;;       b.ne    #0x1e4
;;       b       #0x1a4
;;  1a4: sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w1, [x28]
;;       sub     sp, sp, #0xc
;;       mov     x28, sp
;;       mov     x0, x9
;;       mov     x16, #0
;;       mov     w1, w16
;;       ldur    w2, [x28, #0xc]
;;       bl      #0x398
;;  1cc: add     sp, sp, #0xc
;;       mov     x28, sp
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       ldur    x9, [x28, #0x18]
;;       b       #0x1e8
;;  1e4: and     x0, x0, #0xfffffffffffffffe
;;       cbz     x0, #0x270
;;  1ec: ldur    x16, [x9, #0x50]
;;       ldur    w1, [x16]
;;       ldur    w2, [x0, #0x10]
;;       cmp     w1, w2, uxtx
;;       b.ne    #0x274
;;  200: sub     sp, sp, #8
;;       mov     x28, sp
;;       stur    x0, [x28]
;;       ldur    x3, [x28]
;;       add     sp, sp, #8
;;       mov     x28, sp
;;       ldur    x5, [x3, #0x18]
;;       ldur    x4, [x3, #8]
;;       mov     x0, x5
;;       mov     x1, x9
;;       ldur    w2, [x28]
;;       blr     x4
;;  230: add     sp, sp, #4
;;       mov     x28, sp
;;       ldur    x9, [x28, #0x14]
;;       ldur    w1, [x28]
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       add     w1, w1, w0, uxtx
;;       mov     w0, w1
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  260: .byte   0x1f, 0xc1, 0x00, 0x00
;;  264: .byte   0x1f, 0xc1, 0x00, 0x00
;;  268: .byte   0x1f, 0xc1, 0x00, 0x00
;;  26c: .byte   0x1f, 0xc1, 0x00, 0x00
;;  270: .byte   0x1f, 0xc1, 0x00, 0x00
;;  274: .byte   0x1f, 0xc1, 0x00, 0x00
