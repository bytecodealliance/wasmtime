;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8765000000         	ja	0x83
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 f20f100d5d000000     	movsd	xmm1, qword ptr [rip + 0x5d]
;;      	 49bb000000000000e041 	
;; 				movabs	r11, 0x41e0000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ecf           	ucomisd	xmm1, xmm15
;;      	 0f8315000000         	jae	0x62
;;      	 0f8a32000000         	jp	0x85
;;   53:	 f20f2cc1             	cvttsd2si	eax, xmm1
;;      	 83f800               	cmp	eax, 0
;;      	 0f8d1d000000         	jge	0x7d
;;   60:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 f2410f5cc7           	subsd	xmm0, xmm15
;;      	 f20f2cc0             	cvttsd2si	eax, xmm0
;;      	 83f800               	cmp	eax, 0
;;      	 0f8c10000000         	jl	0x87
;;   77:	 81c000000080         	add	eax, 0x80000000
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   83:	 0f0b                 	ud2	
;;   85:	 0f0b                 	ud2	
;;   87:	 0f0b                 	ud2	
;;   89:	 0000                 	add	byte ptr [rax], al
;;   8b:	 0000                 	add	byte ptr [rax], al
;;   8d:	 0000                 	add	byte ptr [rax], al
;;   8f:	 0000                 	add	byte ptr [rax], al
;;   91:	 0000                 	add	byte ptr [rax], al
;;   93:	 0000                 	add	byte ptr [rax], al
;;   95:	 00f0                 	add	al, dh
