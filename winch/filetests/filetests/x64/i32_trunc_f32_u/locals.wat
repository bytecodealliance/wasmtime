;;! target = "x86_64"

(module
    (func (result i32)
        (local f32)  

        (local.get 0)
        (i32.trunc_f32_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f875e000000         	ja	0x76
;;   18:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;      	 41bb0000004f         	mov	r11d, 0x4f000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f2ecf             	ucomiss	xmm1, xmm15
;;      	 0f8315000000         	jae	0x55
;;      	 0f8a32000000         	jp	0x78
;;   46:	 f30f2cc1             	cvttss2si	eax, xmm1
;;      	 83f800               	cmp	eax, 0
;;      	 0f8d1d000000         	jge	0x70
;;   53:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 f3410f5cc7           	subss	xmm0, xmm15
;;      	 f30f2cc0             	cvttss2si	eax, xmm0
;;      	 83f800               	cmp	eax, 0
;;      	 0f8c10000000         	jl	0x7a
;;   6a:	 81c000000080         	add	eax, 0x80000000
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   76:	 0f0b                 	ud2	
;;   78:	 0f0b                 	ud2	
;;   7a:	 0f0b                 	ud2	
