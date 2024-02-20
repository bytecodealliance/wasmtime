;;! target = "x86_64"

(module
    (func (param f32) (result i32)
        (local.get 0)
        (i32.trunc_f32_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8754000000         	ja	0x6f
;;   1b:	 4883ec10             	sub	rsp, 0x10
;;      	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;      	 f30f2cc0             	cvttss2si	eax, xmm0
;;      	 83f801               	cmp	eax, 1
;;      	 0f812d000000         	jno	0x69
;;   3c:	 0f2ec0               	ucomiss	xmm0, xmm0
;;      	 0f8a2c000000         	jp	0x71
;;   45:	 41bb000000cf         	mov	r11d, 0xcf000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f2ec7             	ucomiss	xmm0, xmm15
;;      	 0f8219000000         	jb	0x73
;;   5a:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 440f2ef8             	ucomiss	xmm15, xmm0
;;      	 0f820c000000         	jb	0x75
;;   69:	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   6f:	 0f0b                 	ud2	
;;   71:	 0f0b                 	ud2	
;;   73:	 0f0b                 	ud2	
;;   75:	 0f0b                 	ud2	
