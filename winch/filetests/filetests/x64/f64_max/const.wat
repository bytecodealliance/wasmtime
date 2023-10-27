;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.max)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f10053c000000     	movsd	xmm0, qword ptr [rip + 0x3c]
;;   14:	 f20f100d3c000000     	movsd	xmm1, qword ptr [rip + 0x3c]
;;   1c:	 660f2ec8             	ucomisd	xmm1, xmm0
;;   20:	 0f8519000000         	jne	0x3f
;;   26:	 0f8a09000000         	jp	0x35
;;   2c:	 660f54c8             	andpd	xmm1, xmm0
;;   30:	 e90e000000           	jmp	0x43
;;   35:	 f20f58c8             	addsd	xmm1, xmm0
;;   39:	 0f8a04000000         	jp	0x43
;;   3f:	 f20f5fc8             	maxsd	xmm1, xmm0
;;   43:	 660f28c1             	movapd	xmm0, xmm1
;;   47:	 4883c408             	add	rsp, 8
;;   4b:	 5d                   	pop	rbp
;;   4c:	 c3                   	ret	
;;   4d:	 0000                 	add	byte ptr [rax], al
;;   4f:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   55:	 99                   	cdq	
;;   56:	 01409a               	add	dword ptr [rax - 0x66], eax
;;   59:	 99                   	cdq	
;;   5a:	 99                   	cdq	
;;   5b:	 99                   	cdq	
;;   5c:	 99                   	cdq	
;;   5d:	 99                   	cdq	
;;   5e:	 f1                   	int1	
