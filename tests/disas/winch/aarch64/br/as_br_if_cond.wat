;;! target = "aarch64"
;;! test = "winch"
(module
    (func (export "as-br_if-cond")
    (block (br_if 0 (br 0)))
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
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
