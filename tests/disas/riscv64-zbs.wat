;;! target = "riscv64"
;;! test = "compile"
;;! flags = "-Ccranelift-has-zbs"

(module
  (func (export "bclr32") (param i32 i32) (result i32)
    (i32.and (local.get 0) (i32.xor (i32.shl (i32.const 1) (local.get 1)) (i32.const -1)))
  )
  (func (export "bclr64") (param i64 i64) (result i64)
    (i64.and (i64.xor (i64.shl (i64.const 1) (local.get 1)) (i64.const -1)) (local.get 0))
  )
  (func (export "bclri32_4") (param i32) (result i32)
    (i32.and (local.get 0) (i32.xor (i32.shl (i32.const 1) (i32.const 4)) (i32.const -1)))
  )
  (func (export "bclri32_20") (param i32) (result i32)
    (i32.and (i32.xor (i32.shl (i32.const 1) (i32.const 20)) (i32.const -1)) (local.get 0))
  )
  (func (export "bclri64_4") (param i64) (result i64)
    (i64.and (local.get 0) (i64.xor (i64.shl (i64.const 1) (i64.const 4)) (i64.const -1)))
  )
  (func (export "bclri64_52") (param i64) (result i64)
    (i64.and (local.get 0) (i64.xor (i64.shl (i64.const 1) (i64.const 52)) (i64.const -1)))
  )

  (func (export "bext32_1") (param i32 i32) (result i32)
    (i32.and (i32.shr_u (local.get 0) (local.get 1)) (i32.const 1))
  )
  (func (export "bext32_2") (param i32 i32) (result i32)
    (i32.and (i32.shr_s (local.get 0) (local.get 1)) (i32.const 1))
  )
  (func (export "bext32_3") (param i32 i32) (result i32)
    (i32.and (i32.const 1) (i32.shr_u (local.get 0) (local.get 1)))
  )
  (func (export "bext32_4") (param i32 i32) (result i32)
    (i32.and (i32.const 1) (i32.shr_s (local.get 0) (local.get 1)))
  )
  (func (export "bext64_1") (param i64 i64) (result i64)
    (i64.and (i64.shr_u (local.get 0) (local.get 1)) (i64.const 1))
  )
  (func (export "bext64_2") (param i64 i64) (result i64)
    (i64.and (i64.shr_s (local.get 0) (local.get 1)) (i64.const 1))
  )
  (func (export "bext64_3") (param i64 i64) (result i64)
    (i64.and (i64.const 1) (i64.shr_u (local.get 0) (local.get 1)))
  )
  (func (export "bext64_4") (param i64 i64) (result i64)
    (i64.and (i64.const 1) (i64.shr_s (local.get 0) (local.get 1)))
  )

  (func (export "bexti32_1") (param i32) (result i32)
    (i32.and (i32.shr_u (local.get 0) (i32.const 10)) (i32.const 1))
  )
  (func (export "bexti32_2") (param i32) (result i32)
    (i32.and (i32.shr_s (local.get 0) (i32.const 20)) (i32.const 1))
  )
  (func (export "bexti32_3") (param i32) (result i32)
    (i32.and (i32.shr_u (i32.const 1) (local.get 0) (i32.const 30)))
  )
  (func (export "bexti32_4") (param i32) (result i32)
    (i32.and (i32.shr_s (i32.const 1) (local.get 0) (i32.const 40)))
  )
  (func (export "bexti64_1") (param i64) (result i64)
    (i64.and (i64.shr_u (local.get 0) (i64.const 10)) (i64.const 1))
  )
  (func (export "bexti64_2") (param i64) (result i64)
    (i64.and (i64.shr_s (local.get 0) (i64.const 20)) (i64.const 1))
  )
  (func (export "bexti64_3") (param i64) (result i64)
    (i64.and (i64.shr_u (i64.const 1) (local.get 0) (i64.const 30)))
  )
  (func (export "bexti64_4") (param i64) (result i64)
    (i64.and (i64.shr_s (i64.const 1) (local.get 0) (i64.const 40)))
  )

  (func (export "binv32_1") (param i32 i32) (result i32)
    (i32.xor (local.get 0) (i32.shl (i32.const 1) (local.get 1)))
  )
  (func (export "binv32_2") (param i32 i32) (result i32)
    (i32.xor (i32.shl (i32.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "binv64_1") (param i64 i64) (result i64)
    (i64.xor (local.get 0) (i64.shl (i64.const 1) (local.get 1)))
  )
  (func (export "binv64_2") (param i64 i64) (result i64)
    (i64.xor (i64.shl (i64.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "binvi32_1") (param i32) (result i32)
    (i32.xor (local.get 0) (i32.shl (i32.const 1) (i32.const 10)))
  )
  (func (export "binvi32_2") (param i32) (result i32)
    (i32.xor (i32.shl (i32.const 1) (i32.const 20)) (local.get 0))
  )
  (func (export "binvi64_1") (param i64) (result i64)
    (i64.xor (local.get 0) (i64.shl (i64.const 1) (i64.const 30)))
  )
  (func (export "binvi64_2") (param i64) (result i64)
    (i64.xor (i64.shl (i64.const 1) (i64.const 40)) (local.get 0))
  )

  (func (export "bset32_1") (param i32 i32) (result i32)
    (i32.or (local.get 0) (i32.shl (i32.const 1) (local.get 1)))
  )
  (func (export "bset32_2") (param i32 i32) (result i32)
    (i32.or (i32.shl (i32.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "bset64_1") (param i64 i64) (result i64)
    (i64.or (local.get 0) (i64.shl (i64.const 1) (local.get 1)))
  )
  (func (export "bset64_2") (param i64 i64) (result i64)
    (i64.or (i64.shl (i64.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "bseti32_1") (param i32) (result i32)
    (i32.or (local.get 0) (i32.shl (i32.const 1) (i32.const 10)))
  )
  (func (export "bseti32_2") (param i32) (result i32)
    (i32.or (i32.shl (i32.const 1) (i32.const 20)) (local.get 0))
  )
  (func (export "bseti64_1") (param i64) (result i64)
    (i64.or (local.get 0) (i64.shl (i64.const 1) (i64.const 30)))
  )
  (func (export "bseti64_2") (param i64) (result i64)
    (i64.or (i64.shl (i64.const 1) (i64.const 40)) (local.get 0))
  )
)
;; wasm[0]::function[0]:
;;    0: addi    sp, sp, -0x10
;;    4: sd      ra, 8(sp)
;;    8: sd      s0, 0(sp)
;;    c: mv      s0, sp
;;   10: andi    a0, a3, 0x1f
;;   14: .byte   0x33, 0x15, 0xa6, 0x48
;;   18: ld      ra, 8(sp)
;;   1c: ld      s0, 0(sp)
;;   20: addi    sp, sp, 0x10
;;   24: ret
;;
;; wasm[0]::function[1]:
;;   28: addi    sp, sp, -0x10
;;   2c: sd      ra, 8(sp)
;;   30: sd      s0, 0(sp)
;;   34: mv      s0, sp
;;   38: .byte   0x33, 0x15, 0xd6, 0x48
;;   3c: ld      ra, 8(sp)
;;   40: ld      s0, 0(sp)
;;   44: addi    sp, sp, 0x10
;;   48: ret
;;
;; wasm[0]::function[2]:
;;   4c: addi    sp, sp, -0x10
;;   50: sd      ra, 8(sp)
;;   54: sd      s0, 0(sp)
;;   58: mv      s0, sp
;;   5c: .byte   0x13, 0x15, 0x46, 0x48
;;   60: ld      ra, 8(sp)
;;   64: ld      s0, 0(sp)
;;   68: addi    sp, sp, 0x10
;;   6c: ret
;;
;; wasm[0]::function[3]:
;;   70: addi    sp, sp, -0x10
;;   74: sd      ra, 8(sp)
;;   78: sd      s0, 0(sp)
;;   7c: mv      s0, sp
;;   80: .byte   0x13, 0x15, 0x46, 0x49
;;   84: ld      ra, 8(sp)
;;   88: ld      s0, 0(sp)
;;   8c: addi    sp, sp, 0x10
;;   90: ret
;;
;; wasm[0]::function[4]:
;;   94: addi    sp, sp, -0x10
;;   98: sd      ra, 8(sp)
;;   9c: sd      s0, 0(sp)
;;   a0: mv      s0, sp
;;   a4: .byte   0x13, 0x15, 0x46, 0x48
;;   a8: ld      ra, 8(sp)
;;   ac: ld      s0, 0(sp)
;;   b0: addi    sp, sp, 0x10
;;   b4: ret
;;
;; wasm[0]::function[5]:
;;   b8: addi    sp, sp, -0x10
;;   bc: sd      ra, 8(sp)
;;   c0: sd      s0, 0(sp)
;;   c4: mv      s0, sp
;;   c8: .byte   0x13, 0x15, 0x46, 0x4b
;;   cc: ld      ra, 8(sp)
;;   d0: ld      s0, 0(sp)
;;   d4: addi    sp, sp, 0x10
;;   d8: ret
;;
;; wasm[0]::function[6]:
;;   dc: addi    sp, sp, -0x10
;;   e0: sd      ra, 8(sp)
;;   e4: sd      s0, 0(sp)
;;   e8: mv      s0, sp
;;   ec: andi    a0, a3, 0x1f
;;   f0: .byte   0x33, 0x55, 0xa6, 0x48
;;   f4: ld      ra, 8(sp)
;;   f8: ld      s0, 0(sp)
;;   fc: addi    sp, sp, 0x10
;;  100: ret
;;
;; wasm[0]::function[7]:
;;  104: addi    sp, sp, -0x10
;;  108: sd      ra, 8(sp)
;;  10c: sd      s0, 0(sp)
;;  110: mv      s0, sp
;;  114: andi    a0, a3, 0x1f
;;  118: .byte   0x33, 0x55, 0xa6, 0x48
;;  11c: ld      ra, 8(sp)
;;  120: ld      s0, 0(sp)
;;  124: addi    sp, sp, 0x10
;;  128: ret
;;
;; wasm[0]::function[8]:
;;  12c: addi    sp, sp, -0x10
;;  130: sd      ra, 8(sp)
;;  134: sd      s0, 0(sp)
;;  138: mv      s0, sp
;;  13c: andi    a0, a3, 0x1f
;;  140: .byte   0x33, 0x55, 0xa6, 0x48
;;  144: ld      ra, 8(sp)
;;  148: ld      s0, 0(sp)
;;  14c: addi    sp, sp, 0x10
;;  150: ret
;;
;; wasm[0]::function[9]:
;;  154: addi    sp, sp, -0x10
;;  158: sd      ra, 8(sp)
;;  15c: sd      s0, 0(sp)
;;  160: mv      s0, sp
;;  164: andi    a0, a3, 0x1f
;;  168: .byte   0x33, 0x55, 0xa6, 0x48
;;  16c: ld      ra, 8(sp)
;;  170: ld      s0, 0(sp)
;;  174: addi    sp, sp, 0x10
;;  178: ret
;;
;; wasm[0]::function[10]:
;;  17c: addi    sp, sp, -0x10
;;  180: sd      ra, 8(sp)
;;  184: sd      s0, 0(sp)
;;  188: mv      s0, sp
;;  18c: .byte   0x33, 0x55, 0xd6, 0x48
;;  190: ld      ra, 8(sp)
;;  194: ld      s0, 0(sp)
;;  198: addi    sp, sp, 0x10
;;  19c: ret
;;
;; wasm[0]::function[11]:
;;  1a0: addi    sp, sp, -0x10
;;  1a4: sd      ra, 8(sp)
;;  1a8: sd      s0, 0(sp)
;;  1ac: mv      s0, sp
;;  1b0: .byte   0x33, 0x55, 0xd6, 0x48
;;  1b4: ld      ra, 8(sp)
;;  1b8: ld      s0, 0(sp)
;;  1bc: addi    sp, sp, 0x10
;;  1c0: ret
;;
;; wasm[0]::function[12]:
;;  1c4: addi    sp, sp, -0x10
;;  1c8: sd      ra, 8(sp)
;;  1cc: sd      s0, 0(sp)
;;  1d0: mv      s0, sp
;;  1d4: .byte   0x33, 0x55, 0xd6, 0x48
;;  1d8: ld      ra, 8(sp)
;;  1dc: ld      s0, 0(sp)
;;  1e0: addi    sp, sp, 0x10
;;  1e4: ret
;;
;; wasm[0]::function[13]:
;;  1e8: addi    sp, sp, -0x10
;;  1ec: sd      ra, 8(sp)
;;  1f0: sd      s0, 0(sp)
;;  1f4: mv      s0, sp
;;  1f8: .byte   0x33, 0x55, 0xd6, 0x48
;;  1fc: ld      ra, 8(sp)
;;  200: ld      s0, 0(sp)
;;  204: addi    sp, sp, 0x10
;;  208: ret
;;
;; wasm[0]::function[14]:
;;  20c: addi    sp, sp, -0x10
;;  210: sd      ra, 8(sp)
;;  214: sd      s0, 0(sp)
;;  218: mv      s0, sp
;;  21c: .byte   0x13, 0x55, 0xa6, 0x48
;;  220: ld      ra, 8(sp)
;;  224: ld      s0, 0(sp)
;;  228: addi    sp, sp, 0x10
;;  22c: ret
;;
;; wasm[0]::function[15]:
;;  230: addi    sp, sp, -0x10
;;  234: sd      ra, 8(sp)
;;  238: sd      s0, 0(sp)
;;  23c: mv      s0, sp
;;  240: .byte   0x13, 0x55, 0x46, 0x49
;;  244: ld      ra, 8(sp)
;;  248: ld      s0, 0(sp)
;;  24c: addi    sp, sp, 0x10
;;  250: ret
;;
;; wasm[0]::function[16]:
;;  254: addi    sp, sp, -0x10
;;  258: sd      ra, 8(sp)
;;  25c: sd      s0, 0(sp)
;;  260: mv      s0, sp
;;  264: .byte   0x13, 0x55, 0xe6, 0x49
;;  268: ld      ra, 8(sp)
;;  26c: ld      s0, 0(sp)
;;  270: addi    sp, sp, 0x10
;;  274: ret
;;
;; wasm[0]::function[17]:
;;  278: addi    sp, sp, -0x10
;;  27c: sd      ra, 8(sp)
;;  280: sd      s0, 0(sp)
;;  284: mv      s0, sp
;;  288: .byte   0x13, 0x55, 0x86, 0x48
;;  28c: ld      ra, 8(sp)
;;  290: ld      s0, 0(sp)
;;  294: addi    sp, sp, 0x10
;;  298: ret
;;
;; wasm[0]::function[18]:
;;  29c: addi    sp, sp, -0x10
;;  2a0: sd      ra, 8(sp)
;;  2a4: sd      s0, 0(sp)
;;  2a8: mv      s0, sp
;;  2ac: .byte   0x13, 0x55, 0xa6, 0x48
;;  2b0: ld      ra, 8(sp)
;;  2b4: ld      s0, 0(sp)
;;  2b8: addi    sp, sp, 0x10
;;  2bc: ret
;;
;; wasm[0]::function[19]:
;;  2c0: addi    sp, sp, -0x10
;;  2c4: sd      ra, 8(sp)
;;  2c8: sd      s0, 0(sp)
;;  2cc: mv      s0, sp
;;  2d0: .byte   0x13, 0x55, 0x46, 0x49
;;  2d4: ld      ra, 8(sp)
;;  2d8: ld      s0, 0(sp)
;;  2dc: addi    sp, sp, 0x10
;;  2e0: ret
;;
;; wasm[0]::function[20]:
;;  2e4: addi    sp, sp, -0x10
;;  2e8: sd      ra, 8(sp)
;;  2ec: sd      s0, 0(sp)
;;  2f0: mv      s0, sp
;;  2f4: .byte   0x13, 0x55, 0xe6, 0x49
;;  2f8: ld      ra, 8(sp)
;;  2fc: ld      s0, 0(sp)
;;  300: addi    sp, sp, 0x10
;;  304: ret
;;
;; wasm[0]::function[21]:
;;  308: addi    sp, sp, -0x10
;;  30c: sd      ra, 8(sp)
;;  310: sd      s0, 0(sp)
;;  314: mv      s0, sp
;;  318: .byte   0x13, 0x55, 0x86, 0x4a
;;  31c: ld      ra, 8(sp)
;;  320: ld      s0, 0(sp)
;;  324: addi    sp, sp, 0x10
;;  328: ret
;;
;; wasm[0]::function[22]:
;;  32c: addi    sp, sp, -0x10
;;  330: sd      ra, 8(sp)
;;  334: sd      s0, 0(sp)
;;  338: mv      s0, sp
;;  33c: andi    a0, a3, 0x1f
;;  340: .byte   0x33, 0x15, 0xa6, 0x68
;;  344: ld      ra, 8(sp)
;;  348: ld      s0, 0(sp)
;;  34c: addi    sp, sp, 0x10
;;  350: ret
;;
;; wasm[0]::function[23]:
;;  354: addi    sp, sp, -0x10
;;  358: sd      ra, 8(sp)
;;  35c: sd      s0, 0(sp)
;;  360: mv      s0, sp
;;  364: andi    a0, a3, 0x1f
;;  368: .byte   0x33, 0x15, 0xa6, 0x68
;;  36c: ld      ra, 8(sp)
;;  370: ld      s0, 0(sp)
;;  374: addi    sp, sp, 0x10
;;  378: ret
;;
;; wasm[0]::function[24]:
;;  37c: addi    sp, sp, -0x10
;;  380: sd      ra, 8(sp)
;;  384: sd      s0, 0(sp)
;;  388: mv      s0, sp
;;  38c: .byte   0x33, 0x15, 0xd6, 0x68
;;  390: ld      ra, 8(sp)
;;  394: ld      s0, 0(sp)
;;  398: addi    sp, sp, 0x10
;;  39c: ret
;;
;; wasm[0]::function[25]:
;;  3a0: addi    sp, sp, -0x10
;;  3a4: sd      ra, 8(sp)
;;  3a8: sd      s0, 0(sp)
;;  3ac: mv      s0, sp
;;  3b0: .byte   0x33, 0x15, 0xd6, 0x68
;;  3b4: ld      ra, 8(sp)
;;  3b8: ld      s0, 0(sp)
;;  3bc: addi    sp, sp, 0x10
;;  3c0: ret
;;
;; wasm[0]::function[26]:
;;  3c4: addi    sp, sp, -0x10
;;  3c8: sd      ra, 8(sp)
;;  3cc: sd      s0, 0(sp)
;;  3d0: mv      s0, sp
;;  3d4: .byte   0x13, 0x15, 0xa6, 0x68
;;  3d8: ld      ra, 8(sp)
;;  3dc: ld      s0, 0(sp)
;;  3e0: addi    sp, sp, 0x10
;;  3e4: ret
;;
;; wasm[0]::function[27]:
;;  3e8: addi    sp, sp, -0x10
;;  3ec: sd      ra, 8(sp)
;;  3f0: sd      s0, 0(sp)
;;  3f4: mv      s0, sp
;;  3f8: .byte   0x13, 0x15, 0x46, 0x69
;;  3fc: ld      ra, 8(sp)
;;  400: ld      s0, 0(sp)
;;  404: addi    sp, sp, 0x10
;;  408: ret
;;
;; wasm[0]::function[28]:
;;  40c: addi    sp, sp, -0x10
;;  410: sd      ra, 8(sp)
;;  414: sd      s0, 0(sp)
;;  418: mv      s0, sp
;;  41c: .byte   0x13, 0x15, 0xe6, 0x69
;;  420: ld      ra, 8(sp)
;;  424: ld      s0, 0(sp)
;;  428: addi    sp, sp, 0x10
;;  42c: ret
;;
;; wasm[0]::function[29]:
;;  430: addi    sp, sp, -0x10
;;  434: sd      ra, 8(sp)
;;  438: sd      s0, 0(sp)
;;  43c: mv      s0, sp
;;  440: .byte   0x13, 0x15, 0x86, 0x6a
;;  444: ld      ra, 8(sp)
;;  448: ld      s0, 0(sp)
;;  44c: addi    sp, sp, 0x10
;;  450: ret
;;
;; wasm[0]::function[30]:
;;  454: addi    sp, sp, -0x10
;;  458: sd      ra, 8(sp)
;;  45c: sd      s0, 0(sp)
;;  460: mv      s0, sp
;;  464: andi    a0, a3, 0x1f
;;  468: .byte   0x33, 0x15, 0xa6, 0x28
;;  46c: ld      ra, 8(sp)
;;  470: ld      s0, 0(sp)
;;  474: addi    sp, sp, 0x10
;;  478: ret
;;
;; wasm[0]::function[31]:
;;  47c: addi    sp, sp, -0x10
;;  480: sd      ra, 8(sp)
;;  484: sd      s0, 0(sp)
;;  488: mv      s0, sp
;;  48c: andi    a0, a3, 0x1f
;;  490: .byte   0x33, 0x15, 0xa6, 0x28
;;  494: ld      ra, 8(sp)
;;  498: ld      s0, 0(sp)
;;  49c: addi    sp, sp, 0x10
;;  4a0: ret
;;
;; wasm[0]::function[32]:
;;  4a4: addi    sp, sp, -0x10
;;  4a8: sd      ra, 8(sp)
;;  4ac: sd      s0, 0(sp)
;;  4b0: mv      s0, sp
;;  4b4: .byte   0x33, 0x15, 0xd6, 0x28
;;  4b8: ld      ra, 8(sp)
;;  4bc: ld      s0, 0(sp)
;;  4c0: addi    sp, sp, 0x10
;;  4c4: ret
;;
;; wasm[0]::function[33]:
;;  4c8: addi    sp, sp, -0x10
;;  4cc: sd      ra, 8(sp)
;;  4d0: sd      s0, 0(sp)
;;  4d4: mv      s0, sp
;;  4d8: .byte   0x33, 0x15, 0xd6, 0x28
;;  4dc: ld      ra, 8(sp)
;;  4e0: ld      s0, 0(sp)
;;  4e4: addi    sp, sp, 0x10
;;  4e8: ret
;;
;; wasm[0]::function[34]:
;;  4ec: addi    sp, sp, -0x10
;;  4f0: sd      ra, 8(sp)
;;  4f4: sd      s0, 0(sp)
;;  4f8: mv      s0, sp
;;  4fc: .byte   0x13, 0x15, 0xa6, 0x28
;;  500: ld      ra, 8(sp)
;;  504: ld      s0, 0(sp)
;;  508: addi    sp, sp, 0x10
;;  50c: ret
;;
;; wasm[0]::function[35]:
;;  510: addi    sp, sp, -0x10
;;  514: sd      ra, 8(sp)
;;  518: sd      s0, 0(sp)
;;  51c: mv      s0, sp
;;  520: .byte   0x13, 0x15, 0x46, 0x29
;;  524: ld      ra, 8(sp)
;;  528: ld      s0, 0(sp)
;;  52c: addi    sp, sp, 0x10
;;  530: ret
;;
;; wasm[0]::function[36]:
;;  534: addi    sp, sp, -0x10
;;  538: sd      ra, 8(sp)
;;  53c: sd      s0, 0(sp)
;;  540: mv      s0, sp
;;  544: .byte   0x13, 0x15, 0xe6, 0x29
;;  548: ld      ra, 8(sp)
;;  54c: ld      s0, 0(sp)
;;  550: addi    sp, sp, 0x10
;;  554: ret
;;
;; wasm[0]::function[37]:
;;  558: addi    sp, sp, -0x10
;;  55c: sd      ra, 8(sp)
;;  560: sd      s0, 0(sp)
;;  564: mv      s0, sp
;;  568: .byte   0x13, 0x15, 0x86, 0x2a
;;  56c: ld      ra, 8(sp)
;;  570: ld      s0, 0(sp)
;;  574: addi    sp, sp, 0x10
;;  578: ret
