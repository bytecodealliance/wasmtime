;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.gt)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8738000000         	ja	0x53
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10052d000000     	movsd	xmm0, qword ptr [rip + 0x2d]
;;      	 f20f100d2d000000     	movsd	xmm1, qword ptr [rip + 0x2d]
;;      	 660f2ec8             	ucomisd	xmm1, xmm0
;;      	 b800000000           	mov	eax, 0
;;      	 400f97c0             	seta	al
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f9bc3             	setnp	r11b
;;      	 4c21d8               	and	rax, r11
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   53:	 0f0b                 	ud2	
;;   55:	 0000                 	add	byte ptr [rax], al
;;   57:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   5d:	 99                   	cdq	
;;   5e:	 01409a               	add	dword ptr [rax - 0x66], eax
;;   61:	 99                   	cdq	
;;   62:	 99                   	cdq	
;;   63:	 99                   	cdq	
;;   64:	 99                   	cdq	
;;   65:	 99                   	cdq	
;;   66:	 f1                   	int1	
