;;! target = "x86_64"

(module
    (func (result i64)
        (f32.const 1.0)
        (i64.trunc_f32_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f30f10054c000000     	movss	xmm0, dword ptr [rip + 0x4c]
;;   14:	 f3480f2cc0           	cvttss2si	rax, xmm0
;;   19:	 4883f801             	cmp	rax, 1
;;   1d:	 0f812d000000         	jno	0x50
;;   23:	 0f2ec0               	ucomiss	xmm0, xmm0
;;   26:	 0f8a2a000000         	jp	0x56
;;   2c:	 41bb000000df         	mov	r11d, 0xdf000000
;;   32:	 66450f6efb           	movd	xmm15, r11d
;;   37:	 410f2ec7             	ucomiss	xmm0, xmm15
;;   3b:	 0f8217000000         	jb	0x58
;;   41:	 66450f57ff           	xorpd	xmm15, xmm15
;;   46:	 440f2ef8             	ucomiss	xmm15, xmm0
;;   4a:	 0f820a000000         	jb	0x5a
;;   50:	 4883c408             	add	rsp, 8
;;   54:	 5d                   	pop	rbp
;;   55:	 c3                   	ret	
;;   56:	 0f0b                 	ud2	
;;   58:	 0f0b                 	ud2	
;;   5a:	 0f0b                 	ud2	
;;   5c:	 0000                 	add	byte ptr [rax], al
;;   5e:	 0000                 	add	byte ptr [rax], al
;;   60:	 0000                 	add	byte ptr [rax], al
