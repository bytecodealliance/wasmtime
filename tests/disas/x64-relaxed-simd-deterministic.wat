;;! target = "x86_64"
;;! test = "compile"
;;! flags = "-Ccranelift-sse42 -Ccranelift-has-avx -Wrelaxed-simd-deterministic"

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
;;       vcmpeqps %xmm0, %xmm0, %xmm5
;;       vandps  %xmm5, %xmm0, %xmm7
;;       vpxor   %xmm7, %xmm5, %xmm1
;;       vcvttps2dq %xmm7, %xmm3
;;       vpand   %xmm1, %xmm3, %xmm5
;;       vpsrad  $0x1f, %xmm5, %xmm7
;;       vpxor   %xmm3, %xmm7, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vxorps  %xmm5, %xmm5, %xmm7
;;       vmaxps  %xmm7, %xmm0, %xmm2
;;       vpcmpeqd %xmm7, %xmm7, %xmm3
;;       vpsrld  $1, %xmm3, %xmm5
;;       vcvtdq2ps %xmm5, %xmm7
;;       vcvttps2dq %xmm2, %xmm1
;;       vsubps  %xmm7, %xmm2, %xmm3
;;       vcmpleps %xmm3, %xmm7, %xmm5
;;       vcvttps2dq %xmm3, %xmm7
;;       vpxor   %xmm5, %xmm7, %xmm2
;;       vpxor   %xmm3, %xmm3, %xmm5
;;       vpmaxsd %xmm5, %xmm2, %xmm7
;;       vpaddd  %xmm1, %xmm7, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vcmpeqpd %xmm0, %xmm0, %xmm5
;;       vandps  0xf(%rip), %xmm5, %xmm7
;;       vminpd  %xmm7, %xmm0, %xmm1
;;       vcvttpd2dq %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;       addb    %al, (%rax)
;;       addb    %al, (%rax)
;;       sarb    $0xff, %bh
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vxorpd  %xmm5, %xmm5, %xmm7
;;       vmaxpd  %xmm7, %xmm0, %xmm1
;;       vminpd  0x1c(%rip), %xmm1, %xmm3
;;       vroundpd $3, %xmm3, %xmm5
;;       vaddpd  0x1e(%rip), %xmm5, %xmm0
;;       vshufps $0x88, %xmm7, %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;       addb    %al, (%rax)
;;       addb    %al, (%rax)
;;       addb    %al, (%rax)
;;       loopne  0xd3
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpmovsxbw %xmm0, %xmm6
;;       vpmovsxbw %xmm1, %xmm7
;;       vpmullw %xmm7, %xmm6, %xmm6
;;       vpalignr $8, %xmm0, %xmm0, %xmm5
;;       vpmovsxbw %xmm5, %xmm7
;;       vpalignr $8, %xmm1, %xmm1, %xmm5
;;       vpmovsxbw %xmm5, %xmm0
;;       vpmullw %xmm0, %xmm7, %xmm7
;;       vphaddw %xmm7, %xmm6, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpmovsxbw %xmm0, %xmm3
;;       vpmovsxbw %xmm1, %xmm4
;;       vpmullw %xmm4, %xmm3, %xmm3
;;       vpalignr $8, %xmm0, %xmm0, %xmm0
;;       vpmovsxbw %xmm0, %xmm4
;;       vpalignr $8, %xmm1, %xmm1, %xmm0
;;       vpmovsxbw %xmm0, %xmm5
;;       vpmullw %xmm5, %xmm4, %xmm1
;;       vphaddw %xmm1, %xmm3, %xmm1
;;       vpmaddwd 0x17(%rip), %xmm1, %xmm1
;;       vpaddd  %xmm2, %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;       addb    %al, (%rax)
;;       addb    %al, (%rax)
;;       addb    %al, (%rax)
;;       addb    %al, (%rax)
;;       addb    %al, (%rax)
;;       addb    %al, (%rax)
;;       addb    %al, (%rax)
;;       addl    %eax, (%rax)
;;       addl    %eax, (%rax)
;;       addl    %eax, (%rax)
;;       addl    %eax, (%rax)
;;       addl    %eax, (%rax)
;;       addl    %eax, (%rax)
;;       addl    %eax, (%rax)
;;       addl    %eax, (%rax)
