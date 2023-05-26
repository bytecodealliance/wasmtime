;;! target = "x86_64"
;;! compile = true
;;! relaxed_simd_deterministic = true
;;! settings = ["enable_simd", "has_avx"]

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
;;   vcmpps xmm3, xmm0, xmm0, 0x0
;;   vandps xmm5, xmm0, xmm3
;;   vpxor xmm7, xmm3, xmm5
;;   vcvttps2dq xmm9, xmm5
;;   vpand xmm11, xmm9, xmm7
;;   vpsrad xmm13, xmm11, $31
;;   vpxor xmm0, xmm13, xmm9
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
;;   uninit xmm3
;;   vxorps xmm5, xmm3, xmm3
;;   vmaxps xmm7, xmm0, xmm5
;;   vpcmpeqd xmm9, xmm5, xmm5
;;   vpsrld xmm11, xmm9, $1
;;   vcvtdq2ps xmm13, xmm11
;;   vcvttps2dq xmm15, xmm7
;;   vsubps xmm1, xmm7, xmm13
;;   vcmpps xmm3, xmm13, xmm1, 0x2
;;   vcvttps2dq xmm5, xmm1
;;   vpxor xmm7, xmm5, xmm3
;;   uninit xmm9
;;   vpxor xmm11, xmm9, xmm9
;;   vpmaxsd xmm13, xmm7, xmm11
;;   vpaddd xmm0, xmm13, xmm15
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
;;   vcmppd xmm3, xmm0, xmm0, 0x0
;;   vandps xmm5, xmm3, const(0)
;;   vminpd xmm7, xmm0, xmm5
;;   vcvttpd2dq xmm0, xmm7
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
;;   uninit xmm3
;;   vxorpd xmm5, xmm3, xmm3
;;   vmaxpd xmm7, xmm0, xmm5
;;   vminpd xmm9, xmm7, const(0)
;;   vroundpd xmm11, xmm9, 3
;;   vaddpd xmm13, xmm11, const(1)
;;   vshufps xmm0, xmm13, xmm5, 0x88
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
;;   vpmovsxbw xmm12, xmm0
;;   vpmovsxbw xmm13, xmm1
;;   vpmullw xmm12, xmm12, xmm13
;;   vpalignr xmm11, xmm0, xmm0, 0x8
;;   vpmovsxbw xmm13, xmm11
;;   vpalignr xmm11, xmm1, xmm1, 0x8
;;   vpmovsxbw xmm14, xmm11
;;   vpmullw xmm13, xmm13, xmm14
;;   vphaddw xmm0, xmm12, xmm13
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
;;   vpmovsxbw xmm15, xmm0
;;   vpmovsxbw xmm3, xmm1
;;   vpmullw xmm15, xmm15, xmm3
;;   vpalignr xmm14, xmm0, xmm0, 0x8
;;   vpmovsxbw xmm0, xmm14
;;   vpalignr xmm14, xmm1, xmm1, 0x8
;;   vpmovsxbw xmm1, xmm14
;;   vpmullw xmm0, xmm0, xmm1
;;   vphaddw xmm15, xmm15, xmm0
;;   vpmaddwd xmm15, xmm15, const(0)
;;   vpaddd xmm0, xmm15, xmm2
;;   jmp label1
;; block1:
;;   mov rsp, rbp
;;   pop rbp
;;   ret
