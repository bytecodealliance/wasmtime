;;! target = "x86_64"
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
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movdqu  0xff4(%rip), %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movdqu  0xfe4(%rip), %xmm5
;;       paddd   %xmm5, %xmm0
;;       pxor    %xmm5, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movdqu  0xfd4(%rip), %xmm6
;;       paddd   %xmm6, %xmm0
;;       movdqu  0xfd8(%rip), %xmm6
;;       pxor    %xmm6, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; .wasmtime.constants:
;; 1000: 11 22 33 44 55 66 77 88 99 aa bb cc dd ee f0 0f
;; 1010: 01 00 00 00 02 00 00 00 03 00 00 00 04 00 00 00
;; 1020: 01 00 00 00 02 00 00 00 03 00 00 00 04 00 00 00
;; 1030: 05 00 00 00 06 00 00 00 07 00 00 00 08 00 00 00
