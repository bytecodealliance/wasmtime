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
;;   vcvttps2dq %xmm5, %xmm1
;;   vpand   %xmm1, %xmm7, %xmm3
;;   vpsrad  %xmm3, $31, %xmm5
;;   vpxor   %xmm5, %xmm1, %xmm0
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
;;   vmaxps  %xmm0, %xmm5, %xmm0
;;   vpcmpeqd %xmm5, %xmm5, %xmm1
;;   vpsrld  %xmm1, $1, %xmm3
;;   vcvtdq2ps %xmm3, %xmm5
;;   vcvttps2dq %xmm0, %xmm7
;;   vsubps  %xmm0, %xmm5, %xmm1
;;   vcmpps  $2, %xmm5, %xmm1, %xmm3
;;   vcvttps2dq %xmm1, %xmm5
;;   vpxor   %xmm5, %xmm3, %xmm0
;;   uninit  %xmm1
;;   vpxor   %xmm1, %xmm1, %xmm3
;;   vpmaxsd %xmm0, %xmm3, %xmm5
;;   vpaddd  %xmm5, %xmm7, %xmm0
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
;;   vminpd  %xmm7, const(0), %xmm1
;;   vroundpd $3, %xmm1, %xmm3
;;   vaddpd  %xmm3, const(1), %xmm6
;;   vshufps $136, %xmm6, %xmm5, %xmm0
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
;;   vpmovsxbw %xmm0, %xmm4
;;   vpmovsxbw %xmm1, %xmm5
;;   vpmullw %xmm4, %xmm5, %xmm4
;;   vpalignr $8, %xmm0, %xmm0, %xmm3
;;   vpmovsxbw %xmm3, %xmm5
;;   vpalignr $8, %xmm1, %xmm1, %xmm3
;;   vpmovsxbw %xmm3, %xmm6
;;   vpmullw %xmm5, %xmm6, %xmm5
;;   vphaddw %xmm4, %xmm5, %xmm0
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
;;   vpmovsxbw %xmm0, %xmm7
;;   vpmovsxbw %xmm1, %xmm3
;;   vpmullw %xmm7, %xmm3, %xmm7
;;   vpalignr $8, %xmm0, %xmm0, %xmm6
;;   vpmovsxbw %xmm6, %xmm0
;;   vpalignr $8, %xmm1, %xmm1, %xmm6
;;   vpmovsxbw %xmm6, %xmm1
;;   vpmullw %xmm0, %xmm1, %xmm0
;;   vphaddw %xmm7, %xmm0, %xmm7
;;   vpmaddwd %xmm7, const(0), %xmm7
;;   vpaddd  %xmm7, %xmm2, %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
