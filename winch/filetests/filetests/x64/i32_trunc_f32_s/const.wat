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
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10054c000000     	movss	xmm0, dword ptr [rip + 0x4c]
;;      	 f30f2cc0             	cvttss2si	eax, xmm0
;;      	 83f801               	cmp	eax, 1
;;      	 0f812d000000         	jno	0x4e
;;   21:	 0f2ec0               	ucomiss	xmm0, xmm0
;;      	 0f8a2a000000         	jp	0x54
;;   2a:	 41bb000000cf         	mov	r11d, 0xcf000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f2ec7             	ucomiss	xmm0, xmm15
;;      	 0f8217000000         	jb	0x56
;;   3f:	 66450f57ff           	xorpd	xmm15, xmm15
;;      	 440f2ef8             	ucomiss	xmm15, xmm0
;;      	 0f820a000000         	jb	0x58
;;   4e:	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   54:	 0f0b                 	ud2	
;;   56:	 0f0b                 	ud2	
;;   58:	 0f0b                 	ud2	
;;   5a:	 0000                 	add	byte ptr [rax], al
;;   5c:	 0000                 	add	byte ptr [rax], al
;;   5e:	 0000                 	add	byte ptr [rax], al
;;   60:	 0000                 	add	byte ptr [rax], al
