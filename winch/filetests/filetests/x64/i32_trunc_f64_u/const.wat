;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f875c000000         	ja	0x74
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f100d5c000000     	movsd	xmm1, qword ptr [rip + 0x5c]
;;      	 49bb000000000000e041 	
;; 				movabs	r11, 0x41e0000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f2ecf           	ucomisd	xmm1, xmm15
;;      	 0f8315000000         	jae	0x53
;;      	 0f8a32000000         	jp	0x76
;;   44:	 f20f2cc1             	cvttsd2si	eax, xmm1
;;      	 83f800               	cmp	eax, 0
;;      	 0f8d1d000000         	jge	0x6e
;;   51:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 f2410f5cc7           	subsd	xmm0, xmm15
;;      	 f20f2cc0             	cvttsd2si	eax, xmm0
;;      	 83f800               	cmp	eax, 0
;;      	 0f8c10000000         	jl	0x78
;;   68:	 81c000000080         	add	eax, 0x80000000
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   74:	 0f0b                 	ud2	
;;   76:	 0f0b                 	ud2	
;;   78:	 0f0b                 	ud2	
;;   7a:	 0000                 	add	byte ptr [rax], al
;;   7c:	 0000                 	add	byte ptr [rax], al
;;   7e:	 0000                 	add	byte ptr [rax], al
;;   80:	 0000                 	add	byte ptr [rax], al
;;   82:	 0000                 	add	byte ptr [rax], al
;;   84:	 0000                 	add	byte ptr [rax], al
