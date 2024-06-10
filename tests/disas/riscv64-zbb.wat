;;! target = "riscv64"
;;! test = "compile"
;;! flags = "-Ccranelift-has-zbb"

(module
  (func (export "rolw") (param i32 i32) (result i32)
    (i32.rotl (local.get 0) (local.get 1)))
  (func (export "rol") (param i64 i64) (result i64)
    (i64.rotl (local.get 0) (local.get 1)))
  (func (export "rolwi") (param i32 ) (result i32)
    (i32.rotl (local.get 0) (i32.const 100)))
  (func (export "roli") (param i64) (result i64)
    (i64.rotl (local.get 0) (i64.const 40)))

  (func (export "rorw") (param i32 i32) (result i32)
    (i32.rotr (local.get 0) (local.get 1)))
  (func (export "ror") (param i64 i64) (result i64)
    (i64.rotr (local.get 0) (local.get 1)))
  (func (export "rorwi") (param i32 ) (result i32)
    (i32.rotr (local.get 0) (i32.const 100)))
  (func (export "rori") (param i64) (result i64)
    (i64.rotr (local.get 0) (i64.const 40)))

  (func (export "xnor32_1") (param i32 i32) (result i32)
    (i32.xor (i32.xor (local.get 0) (local.get 1)) (i32.const -1)))
  (func (export "xnor32_2") (param i32 i32) (result i32)
    (i32.xor (i32.const -1) (i32.xor (local.get 0) (local.get 1))))
  (func (export "xnor64_1") (param i64 i64) (result i64)
    (i64.xor (i64.xor (local.get 0) (local.get 1)) (i64.const -1)))
  (func (export "xnor64_2") (param i64 i64) (result i64)
    (i64.xor (i64.const -1) (i64.xor (local.get 0) (local.get 1))))
)
;; wasm[0]::function[0]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x3b, 0x15, 0xd6, 0x60
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
;;       .byte   0x33, 0x15, 0xd6, 0x60
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
;;       .byte   0x1b, 0x55, 0xc6, 0x61
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[3]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x55, 0x86, 0x61
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[4]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x3b, 0x55, 0xd6, 0x60
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[5]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x33, 0x55, 0xd6, 0x60
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[6]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x1b, 0x55, 0x46, 0x60
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[7]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x55, 0x86, 0x62
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[8]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x33, 0x45, 0xd6, 0x40
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[9]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x33, 0x45, 0xd6, 0x40
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[10]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x33, 0x45, 0xd6, 0x40
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[11]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x33, 0x45, 0xd6, 0x40
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
