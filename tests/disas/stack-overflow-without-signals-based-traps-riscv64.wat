;;! target = "riscv64-unknown-linux-gnu"
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
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[1]:
;;       ld      t6, 8(a0)
;;       ld      t6, 0x10(t6)
;;       addi    t6, t6, 0x30
;;       bgeu    sp, t6, 0xc
;;       auipc   t6, 0
;;       jalr    zero, t6, 0x16c
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       addi    sp, sp, -0x10
;;       sd      s1, 8(sp)
;;       sd      s2, 0(sp)
;;       mv      s1, a3
;;       mv      a1, a0
;;       mv      s2, a2
;;       auipc   ra, 0
;;       jalr    ra, ra, -0x60
;;       mv      a2, s2
;;       mv      a3, s1
;;       add     a0, a2, a3
;;       ld      s1, 8(sp)
;;       ld      s2, 0(sp)
;;       addi    sp, sp, 0x10
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
