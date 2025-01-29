;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const -1)
	(i64.div_s)
    )
)

;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x16, #-1
;;       mov     x0, x16
;;       mov     x16, #-0x8000000000000000
;;       mov     x1, x16
;;       cbz     x0, #0x58
;;   34: cmn     x0, #1
;;       ccmp    x1, #1, #0, eq
;;       b.vs    #0x5c
;;   40: sdiv    x1, x1, x0
;;       mov     x0, x1
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   58: .byte   0x1f, 0xc1, 0x00, 0x00
;;   5c: .byte   0x1f, 0xc1, 0x00, 0x00
