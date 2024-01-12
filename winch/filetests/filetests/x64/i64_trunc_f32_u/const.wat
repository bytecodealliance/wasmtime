;;! target = "x86_64"

(module
    (func (result i64)
        (f32.const 1.0)
        (i64.trunc_f32_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f30f100d5c000000     	movss	xmm1, dword ptr [rip + 0x5c]
;;   14:	 41bb0000005f         	mov	r11d, 0x5f000000
;;   1a:	 66450f6efb           	movd	xmm15, r11d
;;   1f:	 410f2ecf             	ucomiss	xmm1, xmm15
;;   23:	 0f8317000000         	jae	0x40
;;   29:	 0f8a3b000000         	jp	0x6a
;;   2f:	 f3480f2cc1           	cvttss2si	rax, xmm1
;;   34:	 4883f800             	cmp	rax, 0
;;   38:	 0f8d26000000         	jge	0x64
;;   3e:	 0f0b                 	ud2	
;;   40:	 0f28c1               	movaps	xmm0, xmm1
;;   43:	 f3410f5cc7           	subss	xmm0, xmm15
;;   48:	 f3480f2cc0           	cvttss2si	rax, xmm0
;;   4d:	 4883f800             	cmp	rax, 0
;;   51:	 0f8c15000000         	jl	0x6c
;;   57:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   61:	 4c01d8               	add	rax, r11
;;   64:	 4883c408             	add	rsp, 8
;;   68:	 5d                   	pop	rbp
;;   69:	 c3                   	ret	
;;   6a:	 0f0b                 	ud2	
;;   6c:	 0f0b                 	ud2	
;;   6e:	 0000                 	add	byte ptr [rax], al
;;   70:	 0000                 	add	byte ptr [rax], al
