;;! target = "aarch64"
;;! test = "winch"
(module
  (func (export "as-if-else") (param i32 i32) (result i32)
    (block (result i32)
      (if (result i32) (local.get 0)
        (then (local.get 1))
        (else (br 1 (i32.const 4)))
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
;;       stur    w3, [x28]
;;       ldur    w0, [x28, #4]
;;       tst     w0, w0
;;       b.eq    #0x40
;;       b       #0x38
;;   38: ldur    w0, [x28]
;;       b       #0x48
;;   40: mov     x16, #4
;;       mov     w0, w16
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
