;;! target = "x86_64"
;;! compile = true
;;! settings = ["sse41"]

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
;;   cvttps2dq %xmm0, %xmm0
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
;;   uninit  %xmm7
;;   xorps   %xmm7, %xmm7, %xmm7
;;   movdqa  %xmm0, %xmm12
;;   maxps   %xmm12, %xmm7, %xmm12
;;   pcmpeqd %xmm7, %xmm7, %xmm7
;;   psrld   %xmm7, $1, %xmm7
;;   cvtdq2ps %xmm7, %xmm1
;;   cvttps2dq %xmm12, %xmm15
;;   subps   %xmm12, %xmm1, %xmm12
;;   cmpps   $2, %xmm1, %xmm12, %xmm1
;;   cvttps2dq %xmm12, %xmm0
;;   pxor    %xmm0, %xmm1, %xmm0
;;   uninit  %xmm10
;;   pxor    %xmm10, %xmm10, %xmm10
;;   pmaxsd  %xmm0, %xmm10, %xmm0
;;   paddd   %xmm0, %xmm15, %xmm0
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
;;   cvttpd2dq %xmm0, %xmm0
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
;;   uninit  %xmm4
;;   xorpd   %xmm4, %xmm4, %xmm4
;;   movdqa  %xmm0, %xmm8
;;   maxpd   %xmm8, %xmm4, %xmm8
;;   minpd   %xmm8, const(0), %xmm8
;;   roundpd $3, %xmm8, %xmm0
;;   addpd   %xmm0, const(1), %xmm0
;;   shufps  $136, %xmm0, %xmm4, %xmm0
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
;;   movdqa  %xmm1, %xmm4
;;   pmaddubsw %xmm4, %xmm0, %xmm4
;;   movdqa  %xmm4, %xmm0
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
;;   movdqa  %xmm0, %xmm8
;;   movdqa  %xmm1, %xmm0
;;   pmaddubsw %xmm0, %xmm8, %xmm0
;;   pmaddwd %xmm0, const(0), %xmm0
;;   paddd   %xmm0, %xmm2, %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
