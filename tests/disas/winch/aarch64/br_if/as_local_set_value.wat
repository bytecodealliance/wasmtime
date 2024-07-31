;;! target = "aarch64"
;;! test = "winch"
(module
  (func (export "as-local-set-value") (param i32) (result i32)
    (local i32)
    (block (result i32)
      (local.set 0 (br_if 0 (i32.const 17) (local.get 0)))
      (i32.const -1)
    )
  )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x18
;;       mov     x28, sp
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       mov     x16, #0
;;       stur    w16, [x28]
;;       mov     x16, #0
;;       ldur    w1, [x28, #4]
;;       mov     x16, #0x11
;;       mov     w0, w16
;;       tst     w1, w1
;;       b.ne    #0x54
;;       b       #0x48
;;   48: stur    w0, [x28, #4]
;;       orr     x16, xzr, #0xffffffff
;;       mov     w0, w16
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
