;;! target = "x86_64"
;;! compile = true
;;! relaxed_simd_deterministic = true
;;! settings = ["sse42", "has_avx"]

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
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vcmpps  $0, %xmm0, %xmm0, %xmm3
;;   vandps  %xmm0, %xmm3, %xmm5
;;   vpxor   %xmm3, %xmm5, %xmm7
;;   vcvttps2dq %xmm5, %xmm9
;;   vpand   %xmm9, %xmm7, %xmm11
;;   vpsrad  %xmm11, $31, %xmm13
;;   vpxor   %xmm13, %xmm9, %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:1:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   uninit  %xmm3
;;   vxorps  %xmm3, %xmm3, %xmm5
;;   vmaxps  %xmm0, %xmm5, %xmm7
;;   vpcmpeqd %xmm5, %xmm5, %xmm9
;;   vpsrld  %xmm9, $1, %xmm11
;;   vcvtdq2ps %xmm11, %xmm13
;;   vcvttps2dq %xmm7, %xmm15
;;   vsubps  %xmm7, %xmm13, %xmm1
;;   vcmpps  $2, %xmm13, %xmm1, %xmm3
;;   vcvttps2dq %xmm1, %xmm5
;;   vpxor   %xmm5, %xmm3, %xmm7
;;   uninit  %xmm9
;;   vpxor   %xmm9, %xmm9, %xmm11
;;   vpmaxsd %xmm7, %xmm11, %xmm13
;;   vpaddd  %xmm13, %xmm15, %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:2:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vcmppd  $0, %xmm0, %xmm0, %xmm3
;;   vandps  %xmm3, const(0), %xmm5
;;   vminpd  %xmm0, %xmm5, %xmm7
;;   vcvttpd2dq %xmm7, %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:3:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   uninit  %xmm3
;;   vxorpd  %xmm3, %xmm3, %xmm5
;;   vmaxpd  %xmm0, %xmm5, %xmm7
;;   vminpd  %xmm7, const(0), %xmm9
;;   vroundpd $3, %xmm9, %xmm11
;;   vaddpd  %xmm11, const(1), %xmm13
;;   vshufps $136, %xmm13, %xmm5, %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:4:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vpmovsxbw %xmm0, %xmm12
;;   vpmovsxbw %xmm1, %xmm13
;;   vpmullw %xmm12, %xmm13, %xmm12
;;   vpalignr $8, %xmm0, %xmm0, %xmm11
;;   vpmovsxbw %xmm11, %xmm13
;;   vpalignr $8, %xmm1, %xmm1, %xmm11
;;   vpmovsxbw %xmm11, %xmm14
;;   vpmullw %xmm13, %xmm14, %xmm13
;;   vphaddw %xmm12, %xmm13, %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:5:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vpmovsxbw %xmm0, %xmm15
;;   vpmovsxbw %xmm1, %xmm3
;;   vpmullw %xmm15, %xmm3, %xmm15
;;   vpalignr $8, %xmm0, %xmm0, %xmm14
;;   vpmovsxbw %xmm14, %xmm0
;;   vpalignr $8, %xmm1, %xmm1, %xmm14
;;   vpmovsxbw %xmm14, %xmm1
;;   vpmullw %xmm0, %xmm1, %xmm0
;;   vphaddw %xmm15, %xmm0, %xmm15
;;   vpmaddwd %xmm15, const(0), %xmm15
;;   vpaddd  %xmm15, %xmm2, %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
