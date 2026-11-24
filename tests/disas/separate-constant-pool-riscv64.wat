;;! target = "riscv64"
;;! test = "compile"
;;! flags = "-Ccranelift-separate-constant-pool"

(module
  (func (export "one") (result i64)
    i64.const 0xf34bf0a31212003a)
  (func (export "two_uses") (param i64) (result i64)
    (i64.xor
      (i64.add (local.get 0) (i64.const 0xf34bf0a31212003a))
      (i64.const 0xf34bf0a31212003a)))
  (func (export "two_constants") (param i64) (result i64)
    (i64.xor
      (i64.add (local.get 0) (i64.const 0xf34bf0a31212003a))
      (i64.const 0x0123456789abcdef)))
)
;; wasm[0]::function[0]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       auipc   a0, 1
;;       ld      a0, -0x10(a0)
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[1]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       auipc   a0, 1
;;       ld      a0, -0x30(a0)
;;       add     a1, a2, a0
;;       xor     a0, a1, a0
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[2]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       auipc   a0, 1
;;       ld      a0, -0x58(a0)
;;       add     a0, a2, a0
;;       auipc   a1, 1
;;       ld      a1, -0x5c(a1)
;;       xor     a0, a0, a1
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; .wasmtime.constants:
;; 1000: 3a 00 12 12 a3 f0 4b f3 3a 00 12 12 a3 f0 4b f3
;; 1010: 3a 00 12 12 a3 f0 4b f3 ef cd ab 89 67 45 23 01
