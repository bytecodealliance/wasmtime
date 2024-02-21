;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.eq)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f873d000000         	ja	0x5b
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 f20f10052d000000     	movsd	xmm0, qword ptr [rip + 0x2d]
;;      	 f20f100d2d000000     	movsd	xmm1, qword ptr [rip + 0x2d]
;;      	 660f2ec8             	ucomisd	xmm1, xmm0
;;      	 b800000000           	mov	eax, 0
;;      	 400f94c0             	sete	al
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f9bc3             	setnp	r11b
;;      	 4c21d8               	and	rax, r11
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5b:	 0f0b                 	ud2	
;;   5d:	 0000                 	add	byte ptr [rax], al
;;   5f:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   65:	 99                   	cdq	
;;   66:	 01409a               	add	dword ptr [rax - 0x66], eax
;;   69:	 99                   	cdq	
;;   6a:	 99                   	cdq	
;;   6b:	 99                   	cdq	
;;   6c:	 99                   	cdq	
;;   6d:	 99                   	cdq	
;;   6e:	 f1                   	int1	
