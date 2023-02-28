;;! target = "x86_64"
;;! compile = true
;;! relaxed_simd_deterministic = true
;;! settings = ["has_avx=true"]

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
    i16x8.dot_i8x16_i7x16_s
  )

  (func (param v128 v128 v128) (result v128)
    local.get 0
    local.get 1
    local.get 2
    i32x4.dot_i8x16_i7x16_add_s
  )
)

;; function u0:0:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vcmpps  $0 %xmm0, %xmm0, %xmm3
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
;;   xorps   %xmm3, %xmm3, %xmm3
;;   vmaxps  %xmm0, %xmm3, %xmm5
;;   vpcmpeqd %xmm3, %xmm3, %xmm7
;;   vpsrld  %xmm7, $1, %xmm9
;;   vcvtdq2ps %xmm9, %xmm11
;;   vcvttps2dq %xmm5, %xmm13
;;   vsubps  %xmm5, %xmm11, %xmm15
;;   vcmpps  $2 %xmm11, %xmm15, %xmm1
;;   vcvttps2dq %xmm15, %xmm3
;;   vpxor   %xmm3, %xmm1, %xmm5
;;   pxor    %xmm7, %xmm7, %xmm7
;;   vpmaxsd %xmm5, %xmm7, %xmm9
;;   vpaddd  %xmm9, %xmm13, %xmm0
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
;;   vcmppd  $0 %xmm0, %xmm0, %xmm3
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
;;   xorpd   %xmm3, %xmm3, %xmm3
;;   vmaxpd  %xmm0, %xmm3, %xmm5
;;   vminpd  %xmm5, const(0), %xmm7
;;   vroundpd $3, %xmm7, %xmm9
;;   vaddpd  %xmm9, const(1), %xmm11
;;   vshufps $136 %xmm11, %xmm3, %xmm0
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
;;   vpmovsxbw %xmm0, %xmm10
;;   vpmovsxbw %xmm1, %xmm12
;;   vpmullw %xmm10, %xmm12, %xmm14
;;   vpalignr $8 %xmm0, %xmm0, %xmm8
;;   vpmovsxbw %xmm8, %xmm10
;;   vpalignr $8 %xmm1, %xmm1, %xmm12
;;   vpmovsxbw %xmm12, %xmm15
;;   vpmullw %xmm10, %xmm15, %xmm0
;;   vphaddw %xmm14, %xmm0, %xmm0
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
;;   vpmovsxbw %xmm0, %xmm13
;;   vpmovsxbw %xmm1, %xmm15
;;   vpmullw %xmm13, %xmm15, %xmm3
;;   vpalignr $8 %xmm0, %xmm0, %xmm11
;;   vpmovsxbw %xmm11, %xmm13
;;   vpalignr $8 %xmm1, %xmm1, %xmm15
;;   vpmovsxbw %xmm15, %xmm1
;;   vpmullw %xmm13, %xmm1, %xmm4
;;   vphaddw %xmm3, %xmm4, %xmm15
;;   vpmaddwd %xmm15, const(0), %xmm15
;;   vpaddd  %xmm15, %xmm2, %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
