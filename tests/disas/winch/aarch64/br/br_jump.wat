;;! target = "aarch64"
;;! test = "winch"
(module
  (func (;0;) (result i32)
    (local i32)
    local.get 0
    loop ;; label = @1
      local.get 0
      block ;; label = @2
      end
      br 0 (;@1;)
    end
  )
  (export "" (func 0))
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x20
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x8c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       ldur    w16, [x28, #4]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x28]
;;       ldur    w16, [x28, #8]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       b       #0x58
;;   74: add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   8c: .byte   0x1f, 0xc1, 0x00, 0x00
