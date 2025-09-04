;;! target = "riscv64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       ld      a5, 8(a0)
;;       mv      a2, s0
;;       sd      a2, 0x38(a5)
;;       auipc   ra, 0
;;       jalr    ra, ra, -0x3c
;;       addi    a0, zero, 1
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
