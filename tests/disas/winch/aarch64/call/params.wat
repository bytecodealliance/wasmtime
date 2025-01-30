;;! target = "aarch64"
;;! test = "winch"

(module
  (func (export "main") (param i32) (param i32) (result i32)
    (local.get 1)
    (local.get 0)
    (i32.add)

    (call $add (i32.const 1) (i32.const 2) (i32.const 3) (i32.const 4) (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8))

    (local.get 1)
    (local.get 0)
    (i32.add)

    (call $add (i32.const 2) (i32.const 3) (i32.const 4) (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8))
  )

  (func $add (param i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.add)
    (local.get 2)
    (i32.add)
    (local.get 3)
    (i32.add)
    (local.get 4)
    (i32.add)
    (local.get 5)
    (i32.add)
    (local.get 6)
    (i32.add)
    (local.get 7)
    (i32.add)
    (local.get 8)
    (i32.add)
  )
)

;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       stur    w3, [x28]
;;       ldur    w0, [x28, #4]
;;       ldur    w1, [x28]
;;       add     w1, w1, w0, uxtx
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       sub     x28, x28, #0x24
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, x9
;;       ldur    w2, [x28, #0x24]
;;       mov     x16, #1
;;       mov     w3, w16
;;       mov     x16, #2
;;       mov     w4, w16
;;       mov     x16, #3
;;       mov     w5, w16
;;       mov     x16, #4
;;       mov     w6, w16
;;       mov     x16, #5
;;       mov     w7, w16
;;       mov     x16, #6
;;       mov     w16, w16
;;       stur    w16, [x28]
;;       mov     x16, #7
;;       mov     w16, w16
;;       stur    w16, [x28, #8]
;;       mov     x16, #8
;;       mov     w16, w16
;;       stur    w16, [x28, #0x10]
;;       bl      #0x160
;;   a4: add     x28, x28, #0x24
;;       add     x28, x28, #4
;;       ldur    x9, [x28, #0x10]
;;       ldur    w1, [x28, #4]
;;       ldur    w2, [x28]
;;       add     w2, w2, w1, uxtx
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w2, [x28]
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, x9
;;       ldur    w2, [x28, #0x24]
;;       ldur    w3, [x28, #0x20]
;;       mov     x16, #2
;;       mov     w4, w16
;;       mov     x16, #3
;;       mov     w5, w16
;;       mov     x16, #4
;;       mov     w6, w16
;;       mov     x16, #5
;;       mov     w7, w16
;;       mov     x16, #6
;;       mov     w16, w16
;;       stur    w16, [x28]
;;       mov     x16, #7
;;       mov     w16, w16
;;       stur    w16, [x28, #8]
;;       mov     x16, #8
;;       mov     w16, w16
;;       stur    w16, [x28, #0x10]
;;       bl      #0x160
;;  134: add     x28, x28, #0x20
;;       add     x28, x28, #8
;;       ldur    x9, [x28, #0x10]
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[1]::add:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
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
;;       ldur    w0, [x28, #0x10]
;;       ldur    w1, [x28, #0x14]
;;       add     w1, w1, w0, uxtx
;;       ldur    w0, [x28, #0xc]
;;       add     w1, w1, w0, uxtx
;;       ldur    w0, [x28, #8]
;;       add     w1, w1, w0, uxtx
;;       ldur    w0, [x28, #4]
;;       add     w1, w1, w0, uxtx
;;       ldur    w0, [x28]
;;       add     w1, w1, w0, uxtx
;;       ldur    w0, [x29, #0x10]
;;       add     w1, w1, w0, uxtx
;;       ldur    w0, [x29, #0x18]
;;       add     w1, w1, w0, uxtx
;;       ldur    w0, [x29, #0x20]
;;       add     w1, w1, w0, uxtx
;;       mov     w0, w1
;;       add     x28, x28, #0x28
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
