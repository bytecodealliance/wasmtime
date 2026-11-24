;;! target = "aarch64-unknown-linux-gnu"
;;! test = "compile"
;;! flags = "-Osignals-based-traps=n"

;; Without signals-based traps, the stack-limit check is done at the very
;; start of each function, before frame setup, and on overflow tail-calls the
;; `stack_overflow` builtin's trampoline.

(module
  (func $callee)
  (func (param i64 i64) (result i64)
    call $callee
    (i64.add (local.get 0) (local.get 1))
  )
)
;; wasm[0]::function[0]::callee:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[1]:
;;       ldur    x16, [x2, #8]
;;       ldur    x16, [x16, #0x10]
;;       add     x16, x16, #0x30
;;       cmp     sp, x16
;;       b.hs    #0x38
;;       b       #0x150
;;   38: stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       stp     x19, x20, [sp, #-0x10]!
;;       mov     x3, x2
;;       mov     x19, x5
;;       mov     x20, x4
;;       bl      #0
;;   54: mov     x4, x20
;;       mov     x5, x19
;;       add     x2, x4, x5
;;       ldp     x19, x20, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
