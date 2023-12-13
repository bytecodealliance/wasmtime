;;! target = "x86_64"

(module
    (func (result i32)
        (local f32)  

        (local.get 0)
        (i32.trunc_f32_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;   1b:	 41bb0000004f         	mov	r11d, 0x4f000000
;;   21:	 66450f6efb           	movd	xmm15, r11d
;;   26:	 410f2ecf             	ucomiss	xmm1, xmm15
;;   2a:	 0f8315000000         	jae	0x45
;;   30:	 0f8a30000000         	jp	0x66
;;   36:	 f30f2cc1             	cvttss2si	eax, xmm1
;;   3a:	 83f800               	cmp	eax, 0
;;   3d:	 0f8d1d000000         	jge	0x60
;;   43:	 0f0b                 	ud2	
;;   45:	 0f28c1               	movaps	xmm0, xmm1
;;   48:	 f3410f5cc7           	subss	xmm0, xmm15
;;   4d:	 f30f2cc0             	cvttss2si	eax, xmm0
;;   51:	 83f800               	cmp	eax, 0
;;   54:	 0f8c0e000000         	jl	0x68
;;   5a:	 81c000000080         	add	eax, 0x80000000
;;   60:	 4883c410             	add	rsp, 0x10
;;   64:	 5d                   	pop	rbp
;;   65:	 c3                   	ret	
;;   66:	 0f0b                 	ud2	
;;   68:	 0f0b                 	ud2	
