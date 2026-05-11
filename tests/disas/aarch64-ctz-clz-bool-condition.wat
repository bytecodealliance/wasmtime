;;! target = "aarch64"
;;! test = "compile"

;; aarch64 analogue of tests/disas/ctz-clz-bool-condition.wat. Verifies that
;; the bare `if (ctz X)` / `if (clz X)` lowerings collapse to a single
;; `tst`/`cmp` + condition, mirroring the x64 lowering rules.

(module
  ;; ----- ctz, i32 -------------------------------------------------------

  (func $if_ctz_eq0_i32 (param i32) (result i32)
    (i32.eq (i32.ctz (local.get 0)) (i32.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $if_ctz_ne0_i32 (param i32) (result i32)
    (i32.ne (i32.ctz (local.get 0)) (i32.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $if_ctz_bare_i32 (param i32) (result i32)
    (i32.ctz (local.get 0))
    if (result i32) i32.const 100 else i32.const 200 end)

  ;; ----- ctz, i64 -------------------------------------------------------

  (func $if_ctz_eq0_i64 (param i64) (result i32)
    (i64.eq (i64.ctz (local.get 0)) (i64.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $if_ctz_bare_i64 (param i64) (result i32)
    (i64.ctz (local.get 0)) i32.wrap_i64
    if (result i32) i32.const 100 else i32.const 200 end)

  ;; ----- clz, i32 (sign-bit tests) --------------------------------------

  (func $if_clz_eq0_i32 (param i32) (result i32)
    (i32.eq (i32.clz (local.get 0)) (i32.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)
  (func $if_clz_bare_i32 (param i32) (result i32)
    (i32.clz (local.get 0))
    if (result i32) i32.const 100 else i32.const 200 end)

  ;; ----- clz, i64 -------------------------------------------------------

  (func $if_clz_eq0_i64 (param i64) (result i32)
    (i64.eq (i64.clz (local.get 0)) (i64.const 0))
    if (result i32) i32.const 100 else i32.const 200 end)

  ;; ----- negative test: numeric comparison must NOT collapse ------------
  (func $if_ctz_eq4_i32 (param i32) (result i32)
    (i32.eq (i32.ctz (local.get 0)) (i32.const 4))
    if (result i32) i32.const 100 else i32.const 200 end)
)
;; wasm[0]::function[0]::if_ctz_eq0_i32:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       and     w6, w4, #1
;;       cbnz    w6, #0x18
;;   10: mov     w2, #0xc8
;;       b       #0x1c
;;   18: mov     w2, #0x64
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[1]::if_ctz_ne0_i32:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       and     w6, w4, #1
;;       cbz     w6, #0x58
;;   50: mov     w2, #0xc8
;;       b       #0x5c
;;   58: mov     w2, #0x64
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[2]::if_ctz_bare_i32:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       tst     w4, #1
;;       b.eq    #0x98
;;   90: mov     w2, #0xc8
;;       b       #0x9c
;;   98: mov     w2, #0x64
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[3]::if_ctz_eq0_i64:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       and     x6, x4, #1
;;       cbnz    x6, #0xd8
;;   d0: mov     w2, #0xc8
;;       b       #0xdc
;;   d8: mov     w2, #0x64
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[4]::if_ctz_bare_i64:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       tst     x4, #1
;;       b.eq    #0x118
;;  110: mov     w2, #0xc8
;;       b       #0x11c
;;  118: mov     w2, #0x64
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[5]::if_clz_eq0_i32:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       cmp     w4, #0
;;       b.lt    #0x158
;;  150: mov     w2, #0xc8
;;       b       #0x15c
;;  158: mov     w2, #0x64
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[6]::if_clz_bare_i32:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       cmp     w4, #0
;;       b.pl    #0x198
;;  190: mov     w2, #0xc8
;;       b       #0x19c
;;  198: mov     w2, #0x64
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[7]::if_clz_eq0_i64:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       cmp     x4, #0
;;       b.lt    #0x1d8
;;  1d0: mov     w2, #0xc8
;;       b       #0x1dc
;;  1d8: mov     w2, #0x64
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[8]::if_ctz_eq4_i32:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       rbit    w6, w4
;;       clz     w8, w6
;;       cmp     w8, #4
;;       b.eq    #0x220
;;  218: mov     w2, #0xc8
;;       b       #0x224
;;  220: mov     w2, #0x64
;;       ldp     x29, x30, [sp], #0x10
;;       ret
