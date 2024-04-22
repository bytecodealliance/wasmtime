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
;;       vcvttps2dq %xmm5, %xmm1
;;       vpand   %xmm7, %xmm1, %xmm3
;;       vpsrad  $0x1f, %xmm3, %xmm5
;;       vpxor   %xmm1, %xmm5, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vxorps  %xmm3, %xmm3, %xmm5
;;       vmaxps  %xmm5, %xmm0, %xmm0
;;       vpcmpeqd %xmm5, %xmm5, %xmm1
;;       vpsrld  $1, %xmm1, %xmm3
;;       vcvtdq2ps %xmm3, %xmm5
;;       vcvttps2dq %xmm0, %xmm7
;;       vsubps  %xmm5, %xmm0, %xmm1
;;       vcmpleps %xmm1, %xmm5, %xmm3
;;       vcvttps2dq %xmm1, %xmm5
;;       vpxor   %xmm3, %xmm5, %xmm0
;;       vpxor   %xmm1, %xmm1, %xmm3
;;       vpmaxsd %xmm3, %xmm0, %xmm5
;;       vpaddd  %xmm7, %xmm5, %xmm0
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
;;   8e: addb    %al, (%rax)
;;   90: addb    %al, (%rax)
;;   92: sarb    $0xff, %bh
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vxorpd  %xmm3, %xmm3, %xmm5
;;       vmaxpd  %xmm5, %xmm0, %xmm7
;;       vminpd  0x1c(%rip), %xmm7, %xmm1
;;       vroundpd $3, %xmm1, %xmm3
;;       vaddpd  0x1e(%rip), %xmm3, %xmm6
;;       vshufps $0x88, %xmm5, %xmm6, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   cc: addb    %al, (%rax)
;;   ce: addb    %al, (%rax)
;;   d0: addb    %al, (%rax)
;;   d2: loopne  0xd3
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpmovsxbw %xmm0, %xmm4
;;       vpmovsxbw %xmm1, %xmm5
;;       vpmullw %xmm5, %xmm4, %xmm4
;;       vpalignr $8, %xmm0, %xmm0, %xmm3
;;       vpmovsxbw %xmm3, %xmm5
;;       vpalignr $8, %xmm1, %xmm1, %xmm3
;;       vpmovsxbw %xmm3, %xmm6
;;       vpmullw %xmm6, %xmm5, %xmm5
;;       vphaddw %xmm5, %xmm4, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpmovsxbw %xmm0, %xmm7
;;       vpmovsxbw %xmm1, %xmm3
;;       vpmullw %xmm3, %xmm7, %xmm7
;;       vpalignr $8, %xmm0, %xmm0, %xmm6
;;       vpmovsxbw %xmm6, %xmm0
;;       vpalignr $8, %xmm1, %xmm1, %xmm6
;;       vpmovsxbw %xmm6, %xmm1
;;       vpmullw %xmm1, %xmm0, %xmm0
;;       vphaddw %xmm0, %xmm7, %xmm7
;;       vpmaddwd 0x17(%rip), %xmm7, %xmm7
;;       vpaddd  %xmm2, %xmm7, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  172: addb    %al, (%rax)
;;  174: addb    %al, (%rax)
;;  176: addb    %al, (%rax)
;;  178: addb    %al, (%rax)
;;  17a: addb    %al, (%rax)
;;  17c: addb    %al, (%rax)
;;  17e: addb    %al, (%rax)
;;  180: addl    %eax, (%rax)
;;  182: addl    %eax, (%rax)
;;  184: addl    %eax, (%rax)
;;  186: addl    %eax, (%rax)
;;  188: addl    %eax, (%rax)
;;  18a: addl    %eax, (%rax)
;;  18c: addl    %eax, (%rax)
;;  18e: addl    %eax, (%rax)
