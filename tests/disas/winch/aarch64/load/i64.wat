;;! target = "aarch64"
;;! test = "winch"
(module
  (memory 1)
  (func (export "i64_load8_s") (param $i i64) (result i64)
   (i64.store8 (i32.const 8) (local.get $i))
   (i64.load8_s (i32.const 8))
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
;;       stur    x2, [x28]
;;       ldur    x0, [x28]
;;       mov     x16, #8
;;       mov     w1, w16
;;       ldur    x2, [x9, #0x50]
;;       add     x2, x2, x1, uxtx
;;       sturb   w0, [x2]
;;       mov     x16, #8
;;       mov     w0, w16
;;       ldur    x1, [x9, #0x50]
;;       add     x1, x1, x0, uxtx
;;       ldursb  x0, [x1]
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
