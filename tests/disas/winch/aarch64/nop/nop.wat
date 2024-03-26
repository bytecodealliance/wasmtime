;;! target = "aarch64"
;;! test = "winch"

(module
    (func
        nop
    )
)
;; wasm[0]::function[0]:
;;    0: stp     x29, x30, [sp, #-0x10]!
;;    4: mov     x29, sp
;;    8: mov     x28, sp
;;    c: mov     x9, x0
;;   10: sub     sp, sp, #0x10
;;   14: mov     x28, sp
;;   18: stur    x0, [x28, #8]
;;   1c: stur    x1, [x28]
;;   20: add     sp, sp, #0x10
;;   24: mov     x28, sp
;;   28: ldp     x29, x30, [sp], #0x10
;;   2c: ret
