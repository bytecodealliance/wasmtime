;;! target = "x86_64"

(module
    (func (param f32) (result i32)
        (local.get 0)
        (i32.trunc_f32_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;      	 f30f2cc0             	cvttss2si	eax, xmm0
;;      	 83f801               	cmp	eax, 1
;;      	 0f812d000000         	jno	0x52
;;   25:	 0f2ec0               	ucomiss	xmm0, xmm0
;;      	 0f8a2a000000         	jp	0x58
;;   2e:	 41bb000000cf         	mov	r11d, 0xcf000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f2ec7             	ucomiss	xmm0, xmm15
;;      	 0f8217000000         	jb	0x5a
;;   43:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 440f2ef8             	ucomiss	xmm15, xmm0
;;      	 0f820a000000         	jb	0x5c
;;   52:	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   58:	 0f0b                 	ud2	
;;   5a:	 0f0b                 	ud2	
;;   5c:	 0f0b                 	ud2	
