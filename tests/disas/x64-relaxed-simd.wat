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
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       cvttps2dq %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       xorps   %xmm7, %xmm7
;;       maxps   %xmm7, %xmm0
;;       pcmpeqd %xmm7, %xmm7
;;       psrld   $1, %xmm7
;;       cvtdq2ps %xmm7, %xmm1
;;       cvttps2dq %xmm0, %xmm7
;;       subps   %xmm1, %xmm0
;;       cmpleps %xmm0, %xmm1
;;       cvttps2dq %xmm0, %xmm0
;;       pxor    %xmm1, %xmm0
;;       pxor    %xmm2, %xmm2
;;       pmaxsd  %xmm2, %xmm0
;;       paddd   %xmm7, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       cvttpd2dq %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       xorpd   %xmm4, %xmm4
;;       maxpd   %xmm4, %xmm0
;;       minpd   0x1c(%rip), %xmm0
;;       roundpd $3, %xmm0, %xmm0
;;       addpd   0x1e(%rip), %xmm0
;;       shufps  $0x88, %xmm4, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   ab: addb    %al, (%rax)
;;   ad: addb    %al, (%rax)
;;   af: addb    %al, (%rax)
;;   b1: addb    %ah, %al
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movdqa  %xmm0, %xmm5
;;       movdqa  %xmm1, %xmm0
;;       pmaddubsw %xmm5, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pmaddubsw %xmm0, %xmm1
;;       pmaddwd 0xf(%rip), %xmm1
;;       movdqa  %xmm1, %xmm0
;;       paddd   %xmm2, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  11e: addb    %al, (%rax)
;;  120: addl    %eax, (%rax)
;;  122: addl    %eax, (%rax)
;;  124: addl    %eax, (%rax)
;;  126: addl    %eax, (%rax)
;;  128: addl    %eax, (%rax)
;;  12a: addl    %eax, (%rax)
;;  12c: addl    %eax, (%rax)
;;  12e: addl    %eax, (%rax)
