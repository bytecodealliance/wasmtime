;;! target = "x86_64"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.trunc_f32_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f30f100d54000000     	movss	xmm1, dword ptr [rip + 0x54]
;;   14:	 41bb0000004f         	mov	r11d, 0x4f000000
;;   1a:	 66450f6efb           	movd	xmm15, r11d
;;   1f:	 410f2ecf             	ucomiss	xmm1, xmm15
;;   23:	 0f8315000000         	jae	0x3e
;;   29:	 0f8a30000000         	jp	0x5f
;;   2f:	 f30f2cc1             	cvttss2si	eax, xmm1
;;   33:	 83f800               	cmp	eax, 0
;;   36:	 0f8d1d000000         	jge	0x59
;;   3c:	 0f0b                 	ud2	
;;   3e:	 0f28c1               	movaps	xmm0, xmm1
;;   41:	 f3410f5cc7           	subss	xmm0, xmm15
;;   46:	 f30f2cc0             	cvttss2si	eax, xmm0
;;   4a:	 83f800               	cmp	eax, 0
;;   4d:	 0f8c0e000000         	jl	0x61
;;   53:	 81c000000080         	add	eax, 0x80000000
;;   59:	 4883c408             	add	rsp, 8
;;   5d:	 5d                   	pop	rbp
;;   5e:	 c3                   	ret	
;;   5f:	 0f0b                 	ud2	
;;   61:	 0f0b                 	ud2	
;;   63:	 0000                 	add	byte ptr [rax], al
;;   65:	 0000                 	add	byte ptr [rax], al
;;   67:	 0000                 	add	byte ptr [rax], al
