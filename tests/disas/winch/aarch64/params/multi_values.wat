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
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x28
;;       mov     x28, sp
;;       stur    x0, [x28, #0x20]
;;       stur    x1, [x28, #0x18]
;;       stur    w2, [x28, #0x14]
;;       stur    w3, [x28, #0x10]
;;       stur    s0, [x28, #0xc]
;;       stur    s1, [x28, #8]
;;       stur    x4, [x28]
;;       ldur    s0, [x28, #8]
;;       ldur    w16, [x28, #0x14]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w16, [x28]
;;       ldur    w16, [x28, #0x14]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w16, [x28]
;;       ldur    s31, [x28, #0x14]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    s31, [x28]
;;       ldur    x0, [x28, #0xc]
;;       ldur    s31, [x28]
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       stur    s31, [x0]
;;       ldur    w16, [x28]
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       stur    w16, [x0, #4]
;;       ldur    w16, [x28]
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       stur    w16, [x0, #8]
;;       add     sp, sp, #0x28
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
