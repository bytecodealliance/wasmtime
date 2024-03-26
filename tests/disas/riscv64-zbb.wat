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
;;    0: addi    sp, sp, -0x10
;;    4: sd      ra, 8(sp)
;;    8: sd      s0, 0(sp)
;;    c: mv      s0, sp
;;   10: .byte   0x3b, 0x15, 0xd6, 0x60
;;   14: ld      ra, 8(sp)
;;   18: ld      s0, 0(sp)
;;   1c: addi    sp, sp, 0x10
;;   20: ret
;;
;; wasm[0]::function[1]:
;;   24: addi    sp, sp, -0x10
;;   28: sd      ra, 8(sp)
;;   2c: sd      s0, 0(sp)
;;   30: mv      s0, sp
;;   34: .byte   0x33, 0x15, 0xd6, 0x60
;;   38: ld      ra, 8(sp)
;;   3c: ld      s0, 0(sp)
;;   40: addi    sp, sp, 0x10
;;   44: ret
;;
;; wasm[0]::function[2]:
;;   48: addi    sp, sp, -0x10
;;   4c: sd      ra, 8(sp)
;;   50: sd      s0, 0(sp)
;;   54: mv      s0, sp
;;   58: .byte   0x1b, 0x55, 0xc6, 0x61
;;   5c: ld      ra, 8(sp)
;;   60: ld      s0, 0(sp)
;;   64: addi    sp, sp, 0x10
;;   68: ret
;;
;; wasm[0]::function[3]:
;;   6c: addi    sp, sp, -0x10
;;   70: sd      ra, 8(sp)
;;   74: sd      s0, 0(sp)
;;   78: mv      s0, sp
;;   7c: .byte   0x13, 0x55, 0x86, 0x61
;;   80: ld      ra, 8(sp)
;;   84: ld      s0, 0(sp)
;;   88: addi    sp, sp, 0x10
;;   8c: ret
;;
;; wasm[0]::function[4]:
;;   90: addi    sp, sp, -0x10
;;   94: sd      ra, 8(sp)
;;   98: sd      s0, 0(sp)
;;   9c: mv      s0, sp
;;   a0: .byte   0x3b, 0x55, 0xd6, 0x60
;;   a4: ld      ra, 8(sp)
;;   a8: ld      s0, 0(sp)
;;   ac: addi    sp, sp, 0x10
;;   b0: ret
;;
;; wasm[0]::function[5]:
;;   b4: addi    sp, sp, -0x10
;;   b8: sd      ra, 8(sp)
;;   bc: sd      s0, 0(sp)
;;   c0: mv      s0, sp
;;   c4: .byte   0x33, 0x55, 0xd6, 0x60
;;   c8: ld      ra, 8(sp)
;;   cc: ld      s0, 0(sp)
;;   d0: addi    sp, sp, 0x10
;;   d4: ret
;;
;; wasm[0]::function[6]:
;;   d8: addi    sp, sp, -0x10
;;   dc: sd      ra, 8(sp)
;;   e0: sd      s0, 0(sp)
;;   e4: mv      s0, sp
;;   e8: .byte   0x1b, 0x55, 0x46, 0x60
;;   ec: ld      ra, 8(sp)
;;   f0: ld      s0, 0(sp)
;;   f4: addi    sp, sp, 0x10
;;   f8: ret
;;
;; wasm[0]::function[7]:
;;   fc: addi    sp, sp, -0x10
;;  100: sd      ra, 8(sp)
;;  104: sd      s0, 0(sp)
;;  108: mv      s0, sp
;;  10c: .byte   0x13, 0x55, 0x86, 0x62
;;  110: ld      ra, 8(sp)
;;  114: ld      s0, 0(sp)
;;  118: addi    sp, sp, 0x10
;;  11c: ret
;;
;; wasm[0]::function[8]:
;;  120: addi    sp, sp, -0x10
;;  124: sd      ra, 8(sp)
;;  128: sd      s0, 0(sp)
;;  12c: mv      s0, sp
;;  130: .byte   0x33, 0x45, 0xd6, 0x40
;;  134: ld      ra, 8(sp)
;;  138: ld      s0, 0(sp)
;;  13c: addi    sp, sp, 0x10
;;  140: ret
;;
;; wasm[0]::function[9]:
;;  144: addi    sp, sp, -0x10
;;  148: sd      ra, 8(sp)
;;  14c: sd      s0, 0(sp)
;;  150: mv      s0, sp
;;  154: .byte   0x33, 0x45, 0xd6, 0x40
;;  158: ld      ra, 8(sp)
;;  15c: ld      s0, 0(sp)
;;  160: addi    sp, sp, 0x10
;;  164: ret
;;
;; wasm[0]::function[10]:
;;  168: addi    sp, sp, -0x10
;;  16c: sd      ra, 8(sp)
;;  170: sd      s0, 0(sp)
;;  174: mv      s0, sp
;;  178: .byte   0x33, 0x45, 0xd6, 0x40
;;  17c: ld      ra, 8(sp)
;;  180: ld      s0, 0(sp)
;;  184: addi    sp, sp, 0x10
;;  188: ret
;;
;; wasm[0]::function[11]:
;;  18c: addi    sp, sp, -0x10
;;  190: sd      ra, 8(sp)
;;  194: sd      s0, 0(sp)
;;  198: mv      s0, sp
;;  19c: .byte   0x33, 0x45, 0xd6, 0x40
;;  1a0: ld      ra, 8(sp)
;;  1a4: ld      s0, 0(sp)
;;  1a8: addi    sp, sp, 0x10
;;  1ac: ret
