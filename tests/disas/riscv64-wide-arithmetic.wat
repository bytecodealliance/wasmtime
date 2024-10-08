;;! target = "riscv64"
;;! test = "compile"
;;! flags = "-Wwide-arithmetic"

(module
  (func $add128 (param i64 i64 i64 i64) (result i64 i64)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    i64.add128)

  (func $sub128 (param i64 i64 i64 i64) (result i64 i64)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    i64.sub128)

  (func $signed (param i64 i64) (result i64 i64)
    local.get 0
    local.get 1
    i64.mul_wide_s)

  (func $unsigned (param i64 i64) (result i64 i64)
    local.get 0
    local.get 1
    i64.mul_wide_u)

  (func $signed_only_high (param i64 i64) (result i64)
    local.get 0
    local.get 1
    i64.mul_wide_s
    local.set 0
    drop
    local.get 0)

  (func $unsigned_only_high (param i64 i64) (result i64)
    local.get 0
    local.get 1
    i64.mul_wide_u
    local.set 0
    drop
    local.get 0)
)

;; wasm[0]::function[0]::add128:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       add     a0, a2, a4
;;       sltu    a1, a0, a4
;;       add     a3, a3, a5
;;       add     a1, a3, a1
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[1]::sub128:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       sub     a0, a2, a4
;;       sltu    a1, a2, a0
;;       sub     a3, a3, a5
;;       sub     a1, a3, a1
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[2]::signed:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       mul     a0, a2, a3
;;       mulh    a1, a2, a3
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[3]::unsigned:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       mul     a0, a2, a3
;;       mulhu   a1, a2, a3
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[4]::signed_only_high:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       mulh    a0, a2, a3
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[5]::unsigned_only_high:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       mulhu   a0, a2, a3
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
