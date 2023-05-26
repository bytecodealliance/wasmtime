;;! target = "x86_64"
;;! compile = true

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
;;   push rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mov rbp, rsp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   cvttps2dq xmm0, xmm0
;;   jmp label1
;; block1:
;;   mov rsp, rbp
;;   pop rbp
;;   ret
;;
;; function u0:1:
;;   push rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mov rbp, rsp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   uninit xmm7
;;   xorps xmm7, xmm7, xmm7
;;   movdqa xmm12, xmm0
;;   maxps xmm12, xmm12, xmm7
;;   pcmpeqd xmm7, xmm7, xmm7
;;   psrld xmm7, xmm7, $1
;;   cvtdq2ps xmm1, xmm7
;;   cvttps2dq xmm15, xmm12
;;   subps xmm12, xmm12, xmm1
;;   cmpps xmm1, xmm1, xmm12, 0x2
;;   cvttps2dq xmm0, xmm12
;;   pxor xmm0, xmm0, xmm1
;;   uninit xmm10
;;   pxor xmm10, xmm10, xmm10
;;   pmaxsd xmm0, xmm0, xmm10
;;   paddd xmm0, xmm0, xmm15
;;   jmp label1
;; block1:
;;   mov rsp, rbp
;;   pop rbp
;;   ret
;;
;; function u0:2:
;;   push rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mov rbp, rsp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   cvttpd2dq xmm0, xmm0
;;   jmp label1
;; block1:
;;   mov rsp, rbp
;;   pop rbp
;;   ret
;;
;; function u0:3:
;;   push rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mov rbp, rsp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   uninit xmm4
;;   xorpd xmm4, xmm4, xmm4
;;   movdqa xmm8, xmm0
;;   maxpd xmm8, xmm8, xmm4
;;   minpd xmm8, xmm8, const(0)
;;   roundpd xmm0, xmm8, 3
;;   addpd xmm0, xmm0, const(1)
;;   shufps xmm0, xmm0, xmm4, 0x88
;;   jmp label1
;; block1:
;;   mov rsp, rbp
;;   pop rbp
;;   ret
;;
;; function u0:4:
;;   push rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mov rbp, rsp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movdqa xmm4, xmm1
;;   pmaddubsw xmm4, xmm4, xmm0
;;   movdqa xmm0, xmm4
;;   jmp label1
;; block1:
;;   mov rsp, rbp
;;   pop rbp
;;   ret
;;
;; function u0:5:
;;   push rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mov rbp, rsp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movdqa xmm8, xmm0
;;   movdqa xmm0, xmm1
;;   pmaddubsw xmm0, xmm0, xmm8
;;   pmaddwd xmm0, xmm0, const(0)
;;   paddd xmm0, xmm0, xmm2
;;   jmp label1
;; block1:
;;   mov rsp, rbp
;;   pop rbp
;;   ret
