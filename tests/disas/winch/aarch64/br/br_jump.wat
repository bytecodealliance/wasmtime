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
;;       mov     x28, sp
;;       mov     x9, x0
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
;;       b       #0x38
;;   50: add     x28, x28, #0x18
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
