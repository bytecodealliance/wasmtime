;;! target = "x86_64"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.trunc_f32_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f874c000000         	ja	0x64
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10054c000000     	movss	xmm0, dword ptr [rip + 0x4c]
;;      	 f30f2cc0             	cvttss2si	eax, xmm0
;;      	 83f801               	cmp	eax, 1
;;      	 0f812d000000         	jno	0x5e
;;   31:	 0f2ec0               	ucomiss	xmm0, xmm0
;;      	 0f8a2c000000         	jp	0x66
;;   3a:	 41bb000000cf         	mov	r11d, 0xcf000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f2ec7             	ucomiss	xmm0, xmm15
;;      	 0f8219000000         	jb	0x68
;;   4f:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 440f2ef8             	ucomiss	xmm15, xmm0
;;      	 0f820c000000         	jb	0x6a
;;   5e:	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   64:	 0f0b                 	ud2	
;;   66:	 0f0b                 	ud2	
;;   68:	 0f0b                 	ud2	
;;   6a:	 0f0b                 	ud2	
;;   6c:	 0000                 	add	byte ptr [rax], al
;;   6e:	 0000                 	add	byte ptr [rax], al
;;   70:	 0000                 	add	byte ptr [rax], al
