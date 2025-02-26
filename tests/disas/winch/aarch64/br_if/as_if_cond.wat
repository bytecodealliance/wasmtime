;;! target = "aarch64"
;;! test = "winch"
(module
  (func (export "as-if-cond") (param i32) (result i32)
    (block (result i32)
      (if (result i32)
        (br_if 0 (i32.const 1) (local.get 0))
        (then (i32.const 2))
        (else (i32.const 3))
      )
    )
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
;;       ldur    w1, [x28, #4]
;;       mov     x16, #1
;;       mov     w0, w16
;;       tst     w1, w1
;;       b.ne    #0x5c
;;       b       #0x3c
;;   3c: tst     w0, w0
;;       b.eq    #0x54
;;       b       #0x48
;;   48: mov     x16, #2
;;       mov     w0, w16
;;       b       #0x5c
;;   54: mov     x16, #3
;;       mov     w0, w16
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
