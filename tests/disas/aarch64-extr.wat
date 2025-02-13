;;! target = "aarch64"
;;! test = "compile"

(module
  (func $i32_21 (param i32 i32) (result i32)
    local.get 0
    i32.const 11
    i32.shl
    local.get 1
    i32.const 21
    i32.shr_u
    i32.or)
  (func $i32_21_swapped (param i32 i32) (result i32)
    local.get 1
    i32.const 21
    i32.shr_u
    local.get 0
    i32.const 11
    i32.shl
    i32.or)
  (func $i32_11 (param i32 i32) (result i32)
    local.get 0
    i32.const 21
    i32.shl
    local.get 1
    i32.const 11
    i32.shr_u
    i32.or)

  (func $i64_21 (param i64 i64) (result i64)
    local.get 0
    i64.const 43
    i64.shl
    local.get 1
    i64.const 21
    i64.shr_u
    i64.or)
  (func $i64_21_swapped (param i64 i64) (result i64)
    local.get 1
    i64.const 21
    i64.shr_u
    local.get 0
    i64.const 43
    i64.shl
    i64.or)
  (func $i64_11 (param i64 i64) (result i64)
    local.get 0
    i64.const 53
    i64.shl
    local.get 1
    i64.const 11
    i64.shr_u
    i64.or)
)

;; wasm[0]::function[0]::i32_21:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       extr    w2, w5, w4, #0x15
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[1]::i32_21_swapped:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       extr    w2, w5, w4, #0x15
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[2]::i32_11:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       extr    w2, w5, w4, #0xb
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[3]::i64_21:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       extr    x2, x5, x4, #0x15
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[4]::i64_21_swapped:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       extr    x2, x5, x4, #0x15
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[5]::i64_11:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       extr    x2, x5, x4, #0xb
;;       ldp     x29, x30, [sp], #0x10
;;       ret
