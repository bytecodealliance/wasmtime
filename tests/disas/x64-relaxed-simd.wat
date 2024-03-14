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
;;   cvttps2dq %xmm0, %xmm0
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
;;   uninit  %xmm1
;;   xorps   %xmm1, %xmm1, %xmm1
;;   maxps   %xmm0, %xmm1, %xmm0
;;   pcmpeqd %xmm1, %xmm1, %xmm1
;;   psrld   %xmm1, $1, %xmm1
;;   cvtdq2ps %xmm1, %xmm2
;;   cvttps2dq %xmm0, %xmm1
;;   subps   %xmm0, %xmm2, %xmm0
;;   cmpps   $2, %xmm2, %xmm0, %xmm2
;;   cvttps2dq %xmm0, %xmm0
;;   pxor    %xmm0, %xmm2, %xmm0
;;   uninit  %xmm4
;;   pxor    %xmm4, %xmm4, %xmm4
;;   pmaxsd  %xmm0, %xmm4, %xmm0
;;   paddd   %xmm0, %xmm1, %xmm0
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
;;   cvttpd2dq %xmm0, %xmm0
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
;;   uninit  %xmm6
;;   xorpd   %xmm6, %xmm6, %xmm6
;;   maxpd   %xmm0, %xmm6, %xmm0
;;   minpd   %xmm0, const(0), %xmm0
;;   roundpd $3, %xmm0, %xmm0
;;   addpd   %xmm0, const(1), %xmm0
;;   shufps  $136, %xmm0, %xmm6, %xmm0
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
;;   movdqa  %xmm0, %xmm7
;;   movdqa  %xmm1, %xmm0
;;   pmaddubsw %xmm0, %xmm7, %xmm0
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
;;   pmaddubsw %xmm1, %xmm0, %xmm1
;;   pmaddwd %xmm1, const(0), %xmm1
;;   movdqa  %xmm1, %xmm0
;;   paddd   %xmm0, %xmm2, %xmm0
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
