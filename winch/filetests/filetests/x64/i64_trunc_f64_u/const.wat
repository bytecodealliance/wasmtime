;;! target = "x86_64"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.trunc_f64_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f100d64000000     	movsd	xmm1, qword ptr [rip + 0x64]
;;   14:	 49bb000000000000e043 	
;; 				movabs	r11, 0x43e0000000000000
;;   1e:	 664d0f6efb           	movq	xmm15, r11
;;   23:	 66410f2ecf           	ucomisd	xmm1, xmm15
;;   28:	 0f8317000000         	jae	0x45
;;   2e:	 0f8a3b000000         	jp	0x6f
;;   34:	 f2480f2cc1           	cvttsd2si	rax, xmm1
;;   39:	 4883f800             	cmp	rax, 0
;;   3d:	 0f8d26000000         	jge	0x69
;;   43:	 0f0b                 	ud2	
;;   45:	 0f28c1               	movaps	xmm0, xmm1
;;   48:	 f2410f5cc7           	subsd	xmm0, xmm15
;;   4d:	 f2480f2cc0           	cvttsd2si	rax, xmm0
;;   52:	 4883f800             	cmp	rax, 0
;;   56:	 0f8c15000000         	jl	0x71
;;   5c:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   66:	 4c01d8               	add	rax, r11
;;   69:	 4883c408             	add	rsp, 8
;;   6d:	 5d                   	pop	rbp
;;   6e:	 c3                   	ret	
;;   6f:	 0f0b                 	ud2	
;;   71:	 0f0b                 	ud2	
;;   73:	 0000                 	add	byte ptr [rax], al
;;   75:	 0000                 	add	byte ptr [rax], al
;;   77:	 0000                 	add	byte ptr [rax], al
;;   79:	 0000                 	add	byte ptr [rax], al
;;   7b:	 0000                 	add	byte ptr [rax], al
;;   7d:	 00f0                 	add	al, dh
