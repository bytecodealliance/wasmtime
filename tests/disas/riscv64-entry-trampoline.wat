;;! target = "riscv64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       lw      a2, 0(a0)
;;       lui     a3, 0x65727
;;       addi    a3, a3, -0x9d
;;       beq     a2, a3, 8
;;       .byte   0x00, 0x00, 0x00, 0x00
;;       ld      a3, 8(a0)
;;       mv      a4, s0
;;       sd      a4, 0x38(a3)
;;       auipc   ra, 0
;;       jalr    ra, ra, -0x50
;;       addi    a0, zero, 1
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
