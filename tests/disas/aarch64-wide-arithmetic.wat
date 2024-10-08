;;! target = "aarch64"
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
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       adds    x2, x4, x6
;;       adc     x3, x5, x7
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[1]::sub128:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       subs    x2, x4, x6
;;       sbc     x3, x5, x7
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[2]::signed:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mul     x2, x4, x5
;;       smulh   x3, x4, x5
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[3]::unsigned:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mul     x2, x4, x5
;;       umulh   x3, x4, x5
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[4]::signed_only_high:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       smulh   x2, x4, x5
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[5]::unsigned_only_high:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       umulh   x2, x4, x5
;;       ldp     x29, x30, [sp], #0x10
;;       ret
