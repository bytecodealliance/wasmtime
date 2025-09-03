;;! target = "riscv64"
;;! test = 'compile'
;;! filter = '_wasm_call'
;;! objdump = '--funcs all'

(component
  (type $a (resource (rep i32)))
  (core func $f (canon resource.drop $a))

  (core module $m (import "" "" (func (param i32))))
  (core instance (instantiate $m (with "" (instance (export "" (func $f))))))
)

;; component-resource-drop[0]_wasm_call:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       addi    sp, sp, -0x10
;;       sd      s11, 8(sp)
;;       mv      s11, a1
;;       ld      a3, 0x10(a0)
;;       mv      a4, s0
;;       sd      a4, 0x28(a3)
;;       ld      a4, 8(s0)
;;       sd      a4, 0x30(a3)
;;       ld      a5, 8(a0)
;;       ld      a5, 0x10(a5)
;;       mv      a1, zero
;;       slli    a1, a1, 0x20
;;       srai    a1, a1, 0x20
;;       slli    a2, a2, 0x20
;;       srai    a2, a2, 0x20
;;       jalr    a5
;;       addi    a5, zero, -1
;;       beq     a0, a5, 0x1c
;;       ld      s11, 8(sp)
;;       addi    sp, sp, 0x10
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;       mv      a1, s11
;;       ld      a0, 0x10(a1)
;;       ld      a2, 0x198(a0)
;;       mv      a0, a1
;;       jalr    a2
;;       .byte   0x00, 0x00, 0x00, 0x00
