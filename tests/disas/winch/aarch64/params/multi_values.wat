;;! target = "aarch64"
;;! test = "winch"

(module
	(func (export "run") (param i32 i32 f32 f32) (result i32 i32 f32 f32)
		local.get 0
		local.get 1
		local.get 2
		local.get 3
	)
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x1, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x34
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0xd4
;;   2c: mov     x9, x1
;;       sub     x28, x28, #0x28
;;       mov     sp, x28
;;       stur    x1, [x28, #0x20]
;;       stur    x2, [x28, #0x18]
;;       stur    w3, [x28, #0x14]
;;       stur    w4, [x28, #0x10]
;;       stur    s0, [x28, #0xc]
;;       stur    s1, [x28, #8]
;;       stur    x0, [x28]
;;       ldur    s0, [x28, #8]
;;       ldur    w16, [x28, #0x14]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x28]
;;       ldur    w16, [x28, #0x14]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x28]
;;       ldur    s31, [x28, #0x14]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    s31, [x28]
;;       ldur    x0, [x28, #0xc]
;;       ldur    s31, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       stur    s31, [x0]
;;       ldur    w16, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x0, #4]
;;       ldur    w16, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x0, #8]
;;       add     x28, x28, #0x28
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   d4: .byte   0x1f, 0xc1, 0x00, 0x00
