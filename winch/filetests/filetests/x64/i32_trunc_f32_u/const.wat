;;! target = "x86_64"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.trunc_f32_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f100d54000000     	movss	xmm1, dword ptr [rip + 0x54]
;;      	 41bb0000004f         	mov	r11d, 0x4f000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f2ecf             	ucomiss	xmm1, xmm15
;;      	 0f8315000000         	jae	0x3e
;;      	 0f8a30000000         	jp	0x5f
;;   2f:	 f30f2cc1             	cvttss2si	eax, xmm1
;;      	 83f800               	cmp	eax, 0
;;      	 0f8d1d000000         	jge	0x59
;;   3c:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 f3410f5cc7           	subss	xmm0, xmm15
;;      	 f30f2cc0             	cvttss2si	eax, xmm0
;;      	 83f800               	cmp	eax, 0
;;      	 0f8c0e000000         	jl	0x61
;;   53:	 81c000000080         	add	eax, 0x80000000
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5f:	 0f0b                 	ud2	
;;   61:	 0f0b                 	ud2	
;;   63:	 0000                 	add	byte ptr [rax], al
;;   65:	 0000                 	add	byte ptr [rax], al
;;   67:	 0000                 	add	byte ptr [rax], al
