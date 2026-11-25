;;! target = "aarch64"
;;! test = "compile"
;;! flags = "-Ccranelift-separate-constant-pool"

(module
  (func (export "one") (result v128)
    v128.const i8x16 0x11 0x22 0x33 0x44 0x55 0x66 0x77 0x88 0x99 0xaa 0xbb 0xcc 0xdd 0xee 0xf0 0x0f)
  (func (export "two_uses") (param v128) (result v128)
    (v128.xor
      (i32x4.add (local.get 0) (v128.const i32x4 1 2 3 4))
      (v128.const i32x4 1 2 3 4)))
  (func (export "two_constants") (param v128) (result v128)
    (v128.xor
      (i32x4.add (local.get 0) (v128.const i32x4 1 2 3 4))
      (v128.const i32x4 5 6 7 8)))
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       adrp    x1, #0x10000
;;       ldr     q0, [x1]
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[1]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       adrp    x5, #0x10000
;;       ldr     q7, [x5, #0x10]
;;       add     v6.4s, v0.4s, v7.4s
;;       eor     v0.16b, v6.16b, v7.16b
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[2]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       adrp    x7, #0x10000
;;       ldr     q16, [x7, #0x20]
;;       add     v16.4s, v0.4s, v16.4s
;;       adrp    x7, #0x10000
;;       ldr     q17, [x7, #0x30]
;;       eor     v0.16b, v16.16b, v17.16b
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; .wasmtime.constants:
;; 10000: 11 22 33 44 55 66 77 88 99 aa bb cc dd ee f0 0f
;; 10010: 01 00 00 00 02 00 00 00 03 00 00 00 04 00 00 00
;; 10020: 01 00 00 00 02 00 00 00 03 00 00 00 04 00 00 00
;; 10030: 05 00 00 00 06 00 00 00 07 00 00 00 08 00 00 00
