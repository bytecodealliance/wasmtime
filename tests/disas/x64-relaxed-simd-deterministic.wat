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
;;       vcmpeqps %xmm0, %xmm0, %xmm3
;;       vandps  %xmm3, %xmm0, %xmm5
;;       vpxor   %xmm5, %xmm3, %xmm7
;;       vcvttps2dq %xmm5, %xmm0
;;       vpand   %xmm7, %xmm0, %xmm1
;;       vpsrad  $0x1f, %xmm1, %xmm1
;;       vpxor   %xmm0, %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vxorps  %xmm3, %xmm3, %xmm5
;;       vmaxps  %xmm5, %xmm0, %xmm7
;;       vpcmpeqd %xmm5, %xmm5, %xmm0
;;       vpsrld  $1, %xmm0, %xmm0
;;       vcvtdq2ps %xmm0, %xmm1
;;       vcvttps2dq %xmm7, %xmm0
;;       vsubps  %xmm1, %xmm7, %xmm2
;;       vcmpleps %xmm2, %xmm1, %xmm1
;;       vcvttps2dq %xmm2, %xmm2
;;       vpxor   %xmm1, %xmm2, %xmm1
;;       vpxor   %xmm2, %xmm2, %xmm2
;;       vpmaxsd %xmm2, %xmm1, %xmm1
;;       vpaddd  %xmm0, %xmm1, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vcmpeqpd %xmm0, %xmm0, %xmm3
;;       vandps  0xf(%rip), %xmm3, %xmm5
;;       vminpd  %xmm5, %xmm0, %xmm7
;;       vcvttpd2dq %xmm7, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   9e: addb    %al, (%rax)
;;   a0: addb    %al, (%rax)
;;   a2: sarb    $0xff, %bh
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vxorpd  %xmm3, %xmm3, %xmm5
;;       vmaxpd  %xmm5, %xmm0, %xmm7
;;       vminpd  0x1c(%rip), %xmm7, %xmm0
;;       vroundpd $3, %xmm0, %xmm0
;;       vaddpd  0x1e(%rip), %xmm0, %xmm0
;;       vshufps $0x88, %xmm5, %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   ec: addb    %al, (%rax)
;;   ee: addb    %al, (%rax)
;;   f0: addb    %al, (%rax)
;;   f2: loopne  0xf3
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpmovsxbw %xmm0, %xmm2
;;       vpmovsxbw %xmm1, %xmm3
;;       vpmullw %xmm3, %xmm2, %xmm2
;;       vpalignr $8, %xmm0, %xmm0, %xmm0
;;       vpmovsxbw %xmm0, %xmm0
;;       vpalignr $8, %xmm1, %xmm1, %xmm1
;;       vpmovsxbw %xmm1, %xmm1
;;       vpmullw %xmm1, %xmm0, %xmm0
;;       vphaddw %xmm0, %xmm2, %xmm0
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
;;       vpmovsxbw %xmm0, %xmm0
;;       vpalignr $8, %xmm1, %xmm1, %xmm1
;;       vpmovsxbw %xmm1, %xmm1
;;       vpmullw %xmm1, %xmm0, %xmm0
;;       vphaddw %xmm0, %xmm3, %xmm0
;;       vpmaddwd 0x17(%rip), %xmm0, %xmm0
;;       vpaddd  %xmm2, %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  1a2: addb    %al, (%rax)
;;  1a4: addb    %al, (%rax)
;;  1a6: addb    %al, (%rax)
;;  1a8: addb    %al, (%rax)
;;  1aa: addb    %al, (%rax)
;;  1ac: addb    %al, (%rax)
;;  1ae: addb    %al, (%rax)
;;  1b0: addl    %eax, (%rax)
;;  1b2: addl    %eax, (%rax)
;;  1b4: addl    %eax, (%rax)
;;  1b6: addl    %eax, (%rax)
;;  1b8: addl    %eax, (%rax)
;;  1ba: addl    %eax, (%rax)
;;  1bc: addl    %eax, (%rax)
;;  1be: addl    %eax, (%rax)
