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

;; function u0:0:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   movq    8(%rdi), %r10
;;   movq    0(%r10), %r10
;;   cmpq    %rsp, %r10
;;   jnbe #trap=stk_ovf
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   vcmpps  $0, %xmm0, %xmm0, %xmm5
;;   vandps  %xmm0, %xmm5, %xmm7
;;   vpxor   %xmm5, %xmm7, %xmm1
;;   vcvttps2dq %xmm7, %xmm3
;;   vpand   %xmm3, %xmm1, %xmm5
;;   vpsrad  %xmm5, $31, %xmm7
;;   vpxor   %xmm7, %xmm3, %xmm0
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:1:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   movq    8(%rdi), %r10
;;   movq    0(%r10), %r10
;;   cmpq    %rsp, %r10
;;   jnbe #trap=stk_ovf
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   uninit  %xmm5
;;   vxorps  %xmm5, %xmm5, %xmm7
;;   vmaxps  %xmm0, %xmm7, %xmm2
;;   vpcmpeqd %xmm7, %xmm7, %xmm3
;;   vpsrld  %xmm3, $1, %xmm5
;;   vcvtdq2ps %xmm5, %xmm7
;;   vcvttps2dq %xmm2, %xmm1
;;   vsubps  %xmm2, %xmm7, %xmm3
;;   vcmpps  $2, %xmm7, %xmm3, %xmm5
;;   vcvttps2dq %xmm3, %xmm7
;;   vpxor   %xmm7, %xmm5, %xmm2
;;   uninit  %xmm3
;;   vpxor   %xmm3, %xmm3, %xmm5
;;   vpmaxsd %xmm2, %xmm5, %xmm7
;;   vpaddd  %xmm7, %xmm1, %xmm0
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:2:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   movq    8(%rdi), %r10
;;   movq    0(%r10), %r10
;;   cmpq    %rsp, %r10
;;   jnbe #trap=stk_ovf
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   vcmppd  $0, %xmm0, %xmm0, %xmm5
;;   vandps  %xmm5, const(0), %xmm7
;;   vminpd  %xmm0, %xmm7, %xmm1
;;   vcvttpd2dq %xmm1, %xmm0
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:3:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   movq    8(%rdi), %r10
;;   movq    0(%r10), %r10
;;   cmpq    %rsp, %r10
;;   jnbe #trap=stk_ovf
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   uninit  %xmm5
;;   vxorpd  %xmm5, %xmm5, %xmm7
;;   vmaxpd  %xmm0, %xmm7, %xmm1
;;   vminpd  %xmm1, const(0), %xmm3
;;   vroundpd $3, %xmm3, %xmm5
;;   vaddpd  %xmm5, const(1), %xmm0
;;   vshufps $136, %xmm0, %xmm7, %xmm0
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:4:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   movq    8(%rdi), %r10
;;   movq    0(%r10), %r10
;;   cmpq    %rsp, %r10
;;   jnbe #trap=stk_ovf
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   vpmovsxbw %xmm0, %xmm6
;;   vpmovsxbw %xmm1, %xmm7
;;   vpmullw %xmm6, %xmm7, %xmm6
;;   vpalignr $8, %xmm0, %xmm0, %xmm5
;;   vpmovsxbw %xmm5, %xmm7
;;   vpalignr $8, %xmm1, %xmm1, %xmm5
;;   vpmovsxbw %xmm5, %xmm0
;;   vpmullw %xmm7, %xmm0, %xmm7
;;   vphaddw %xmm6, %xmm7, %xmm0
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:5:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   movq    8(%rdi), %r10
;;   movq    0(%r10), %r10
;;   cmpq    %rsp, %r10
;;   jnbe #trap=stk_ovf
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   vpmovsxbw %xmm0, %xmm3
;;   vpmovsxbw %xmm1, %xmm4
;;   vpmullw %xmm3, %xmm4, %xmm3
;;   vpalignr $8, %xmm0, %xmm0, %xmm0
;;   vpmovsxbw %xmm0, %xmm4
;;   vpalignr $8, %xmm1, %xmm1, %xmm0
;;   vpmovsxbw %xmm0, %xmm5
;;   vpmullw %xmm4, %xmm5, %xmm1
;;   vphaddw %xmm3, %xmm1, %xmm1
;;   vpmaddwd %xmm1, const(0), %xmm1
;;   vpaddd  %xmm1, %xmm2, %xmm0
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
