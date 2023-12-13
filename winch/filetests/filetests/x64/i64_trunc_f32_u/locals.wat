;;! target = "x86_64"

(module
    (func (result i64)
        (local f32)  

        (local.get 0)
        (i64.trunc_f32_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;   1b:	 41bb0000005f         	mov	r11d, 0x5f000000
;;   21:	 66450f6efb           	movd	xmm15, r11d
;;   26:	 410f2ecf             	ucomiss	xmm1, xmm15
;;   2a:	 0f8317000000         	jae	0x47
;;   30:	 0f8a3b000000         	jp	0x71
;;   36:	 f3480f2cc1           	cvttss2si	rax, xmm1
;;   3b:	 4883f800             	cmp	rax, 0
;;   3f:	 0f8d26000000         	jge	0x6b
;;   45:	 0f0b                 	ud2	
;;   47:	 0f28c1               	movaps	xmm0, xmm1
;;   4a:	 f3410f5cc7           	subss	xmm0, xmm15
;;   4f:	 f3480f2cc0           	cvttss2si	rax, xmm0
;;   54:	 4883f800             	cmp	rax, 0
;;   58:	 0f8c15000000         	jl	0x73
;;   5e:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   68:	 4c01d8               	add	rax, r11
;;   6b:	 4883c410             	add	rsp, 0x10
;;   6f:	 5d                   	pop	rbp
;;   70:	 c3                   	ret	
;;   71:	 0f0b                 	ud2	
;;   73:	 0f0b                 	ud2	
