;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.max)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8745000000         	ja	0x5d
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10053c000000     	movsd	xmm0, qword ptr [rip + 0x3c]
;;      	 f20f100d3c000000     	movsd	xmm1, qword ptr [rip + 0x3c]
;;      	 660f2ec8             	ucomisd	xmm1, xmm0
;;      	 0f8519000000         	jne	0x4f
;;      	 0f8a09000000         	jp	0x45
;;   3c:	 660f54c8             	andpd	xmm1, xmm0
;;      	 e90e000000           	jmp	0x53
;;   45:	 f20f58c8             	addsd	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x53
;;   4f:	 f20f5fc8             	maxsd	xmm1, xmm0
;;      	 660f28c1             	movapd	xmm0, xmm1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5d:	 0f0b                 	ud2	
;;   5f:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   65:	 99                   	cdq	
;;   66:	 01409a               	add	dword ptr [rax - 0x66], eax
;;   69:	 99                   	cdq	
;;   6a:	 99                   	cdq	
;;   6b:	 99                   	cdq	
;;   6c:	 99                   	cdq	
;;   6d:	 99                   	cdq	
;;   6e:	 f1                   	int1	
