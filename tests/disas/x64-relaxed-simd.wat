;;! target = "x86_64"
;;! test = "compile"
;;! flags = "-Ccranelift-sse41"

(module
  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f32x4_s
  )

  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f32x4_u
  )

  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f64x2_s_zero
  )

  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f64x2_u_zero
  )

  (func (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i16x8.relaxed_dot_i8x16_i7x16_s
  )

  (func (param v128 v128 v128) (result v128)
    local.get 0
    local.get 1
    local.get 2
    i32x4.relaxed_dot_i8x16_i7x16_add_s
  )
)

;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: cvttps2dq %xmm0, %xmm0
;;    8: movq    %rbp, %rsp
;;    b: popq    %rbp
;;    c: retq
;;
;; wasm[0]::function[1]:
;;   10: pushq   %rbp
;;   11: movq    %rsp, %rbp
;;   14: xorps   %xmm1, %xmm1
;;   17: maxps   %xmm1, %xmm0
;;   1a: pcmpeqd %xmm1, %xmm1
;;   1e: psrld   $1, %xmm1
;;   23: cvtdq2ps %xmm1, %xmm2
;;   26: cvttps2dq %xmm0, %xmm1
;;   2a: subps   %xmm2, %xmm0
;;   2d: cmpleps %xmm0, %xmm2
;;   31: cvttps2dq %xmm0, %xmm0
;;   35: pxor    %xmm2, %xmm0
;;   39: pxor    %xmm4, %xmm4
;;   3d: pmaxsd  %xmm4, %xmm0
;;   42: paddd   %xmm1, %xmm0
;;   46: movq    %rbp, %rsp
;;   49: popq    %rbp
;;   4a: retq
;;
;; wasm[0]::function[2]:
;;   50: pushq   %rbp
;;   51: movq    %rsp, %rbp
;;   54: cvttpd2dq %xmm0, %xmm0
;;   58: movq    %rbp, %rsp
;;   5b: popq    %rbp
;;   5c: retq
;;
;; wasm[0]::function[3]:
;;   60: pushq   %rbp
;;   61: movq    %rsp, %rbp
;;   64: xorpd   %xmm6, %xmm6
;;   68: maxpd   %xmm6, %xmm0
;;   6c: minpd   0x1c(%rip), %xmm0
;;   74: roundpd $3, %xmm0, %xmm0
;;   7a: addpd   0x1e(%rip), %xmm0
;;   82: shufps  $0x88, %xmm6, %xmm0
;;   86: movq    %rbp, %rsp
;;   89: popq    %rbp
;;   8a: retq
;;   8b: addb    %al, (%rax)
;;   8d: addb    %al, (%rax)
;;   8f: addb    %al, (%rax)
;;   91: addb    %ah, %al
;;
;; wasm[0]::function[4]:
;;   b0: pushq   %rbp
;;   b1: movq    %rsp, %rbp
;;   b4: movdqa  %xmm0, %xmm7
;;   b8: movdqa  %xmm1, %xmm0
;;   bc: pmaddubsw %xmm7, %xmm0
;;   c1: movq    %rbp, %rsp
;;   c4: popq    %rbp
;;   c5: retq
;;
;; wasm[0]::function[5]:
;;   d0: pushq   %rbp
;;   d1: movq    %rsp, %rbp
;;   d4: pmaddubsw %xmm0, %xmm1
;;   d9: pmaddwd 0xf(%rip), %xmm1
;;   e1: movdqa  %xmm1, %xmm0
;;   e5: paddd   %xmm2, %xmm0
;;   e9: movq    %rbp, %rsp
;;   ec: popq    %rbp
;;   ed: retq
;;   ee: addb    %al, (%rax)
;;   f0: addl    %eax, (%rax)
;;   f2: addl    %eax, (%rax)
;;   f4: addl    %eax, (%rax)
;;   f6: addl    %eax, (%rax)
;;   f8: addl    %eax, (%rax)
;;   fa: addl    %eax, (%rax)
;;   fc: addl    %eax, (%rax)
;;   fe: addl    %eax, (%rax)
