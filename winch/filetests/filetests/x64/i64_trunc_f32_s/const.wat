;;! target = "x86_64"

(module
    (func (result i64)
        (f32.const 1.0)
        (i64.trunc_f32_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8752000000         	ja	0x6d
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10054d000000     	movss	xmm0, dword ptr [rip + 0x4d]
;;      	 f3480f2cc0           	cvttss2si	rax, xmm0
;;      	 4883f801             	cmp	rax, 1
;;      	 0f812d000000         	jno	0x67
;;   3a:	 0f2ec0               	ucomiss	xmm0, xmm0
;;      	 0f8a2c000000         	jp	0x6f
;;   43:	 41bb000000df         	mov	r11d, 0xdf000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f2ec7             	ucomiss	xmm0, xmm15
;;      	 0f8219000000         	jb	0x71
;;   58:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 440f2ef8             	ucomiss	xmm15, xmm0
;;      	 0f820c000000         	jb	0x73
;;   67:	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   6d:	 0f0b                 	ud2	
;;   6f:	 0f0b                 	ud2	
;;   71:	 0f0b                 	ud2	
;;   73:	 0f0b                 	ud2	
;;   75:	 0000                 	add	byte ptr [rax], al
;;   77:	 0000                 	add	byte ptr [rax], al
