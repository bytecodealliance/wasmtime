;;! target = "x86_64"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.trunc_f64_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8767000000         	ja	0x7f
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f100d64000000     	movsd	xmm1, qword ptr [rip + 0x64]
;;      	 49bb000000000000e043 	
;; 				movabs	r11, 0x43e0000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ecf           	ucomisd	xmm1, xmm15
;;      	 0f8317000000         	jae	0x55
;;      	 0f8a3d000000         	jp	0x81
;;   44:	 f2480f2cc1           	cvttsd2si	rax, xmm1
;;      	 4883f800             	cmp	rax, 0
;;      	 0f8d26000000         	jge	0x79
;;   53:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 f2410f5cc7           	subsd	xmm0, xmm15
;;      	 f2480f2cc0           	cvttsd2si	rax, xmm0
;;      	 4883f800             	cmp	rax, 0
;;      	 0f8c17000000         	jl	0x83
;;   6c:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;      	 4c01d8               	add	rax, r11
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   7f:	 0f0b                 	ud2	
;;   81:	 0f0b                 	ud2	
;;   83:	 0f0b                 	ud2	
;;   85:	 0000                 	add	byte ptr [rax], al
;;   87:	 0000                 	add	byte ptr [rax], al
;;   89:	 0000                 	add	byte ptr [rax], al
;;   8b:	 0000                 	add	byte ptr [rax], al
;;   8d:	 00f0                 	add	al, dh
