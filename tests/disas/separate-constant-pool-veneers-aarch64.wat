;;! target = "aarch64"
;;! test = "compile"
;;! flags = "-Ccranelift-separate-constant-pool -Ccranelift-wasmtime-linkopt-force-jump-veneer=true"

(module
  (func $a (export "a") (result i32)
    (i32.add
      (i32x4.extract_lane 0 (v128.const i32x4 1 2 3 4))
      (call $b)))
  (func $b (result i32)
    (i32.add
      (i32x4.extract_lane 1 (v128.const i32x4 5 10 15 20))
      (call $c)))
  (func $c (result i32)
    (i32x4.extract_lane 2 (v128.const i32x4 7 8 100 9)))
)
;; wasm[0]::function[0]::a:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       ldur    x16, [x2, #8]
;;       ldur    x16, [x16, #0x10]
;;       add     x16, x16, #0x10
;;       cmp     sp, x16
;;       b.lo    #0x3c
;;   1c: mov     x3, x2
;;       bl      #0x80
;;   24: adrp    x6, #0x10000
;;       ldr     q16, [x6]
;;       mov     w7, v16.s[0]
;;       add     w2, w7, w2
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   3c: udf     #0xc11f
;;
;; wasm[0]::function[1]::b:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       ldur    x16, [x2, #8]
;;       ldur    x16, [x16, #0x10]
;;       add     x16, x16, #0x10
;;       cmp     sp, x16
;;       b.lo    #0x7c
;;   5c: mov     x3, x2
;;       bl      #0xbc
;;   64: adrp    x6, #0x10000
;;       ldr     q16, [x6, #0x10]
;;       mov     w7, v16.s[1]
;;       add     w2, w7, w2
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   7c: udf     #0xc11f
;;
;; wasm[0]::function[2]::c:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       adrp    x2, #0x10000
;;       ldr     q4, [x2, #0x20]
;;       mov     w2, v4.s[2]
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; .wasmtime.constants:
;; 10000: 01 00 00 00 02 00 00 00 03 00 00 00 04 00 00 00
;; 10010: 05 00 00 00 0a 00 00 00 0f 00 00 00 14 00 00 00
;; 10020: 07 00 00 00 08 00 00 00 64 00 00 00 09 00 00 00
