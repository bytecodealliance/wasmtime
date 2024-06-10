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
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       andi    a4, a3, 0x1f
;;       .byte   0x33, 0x15, 0xe6, 0x48
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
;;       .byte   0x33, 0x15, 0xd6, 0x48
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
;;       .byte   0x13, 0x15, 0x46, 0x48
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
;;       .byte   0x13, 0x15, 0x46, 0x49
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
;;       .byte   0x13, 0x15, 0x46, 0x48
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
;;       .byte   0x13, 0x15, 0x46, 0x4b
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
;;       andi    a4, a3, 0x1f
;;       .byte   0x33, 0x55, 0xe6, 0x48
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
;;       andi    a4, a3, 0x1f
;;       .byte   0x33, 0x55, 0xe6, 0x48
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
;;       andi    a4, a3, 0x1f
;;       .byte   0x33, 0x55, 0xe6, 0x48
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
;;       andi    a4, a3, 0x1f
;;       .byte   0x33, 0x55, 0xe6, 0x48
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
;;       .byte   0x33, 0x55, 0xd6, 0x48
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
;;       .byte   0x33, 0x55, 0xd6, 0x48
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[12]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x33, 0x55, 0xd6, 0x48
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[13]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x33, 0x55, 0xd6, 0x48
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[14]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x55, 0xa6, 0x48
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[15]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x55, 0x46, 0x49
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[16]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x55, 0xe6, 0x49
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[17]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x55, 0x86, 0x48
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[18]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x55, 0xa6, 0x48
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[19]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x55, 0x46, 0x49
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[20]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x55, 0xe6, 0x49
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[21]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x55, 0x86, 0x4a
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[22]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       andi    a4, a3, 0x1f
;;       .byte   0x33, 0x15, 0xe6, 0x68
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[23]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       andi    a4, a3, 0x1f
;;       .byte   0x33, 0x15, 0xe6, 0x68
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[24]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x33, 0x15, 0xd6, 0x68
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[25]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x33, 0x15, 0xd6, 0x68
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[26]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x15, 0xa6, 0x68
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[27]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x15, 0x46, 0x69
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[28]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x15, 0xe6, 0x69
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[29]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x15, 0x86, 0x6a
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[30]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       andi    a4, a3, 0x1f
;;       .byte   0x33, 0x15, 0xe6, 0x28
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[31]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       andi    a4, a3, 0x1f
;;       .byte   0x33, 0x15, 0xe6, 0x28
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[32]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x33, 0x15, 0xd6, 0x28
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[33]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x33, 0x15, 0xd6, 0x28
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[34]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x15, 0xa6, 0x28
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[35]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x15, 0x46, 0x29
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[36]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x15, 0xe6, 0x29
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[37]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       .byte   0x13, 0x15, 0x86, 0x2a
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
