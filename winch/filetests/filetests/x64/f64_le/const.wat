;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.le)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8727000000         	ja	0x3f
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f100524000000     	movsd	xmm0, qword ptr [rip + 0x24]
;;      	 f20f100d24000000     	movsd	xmm1, qword ptr [rip + 0x24]
;;      	 660f2ec1             	ucomisd	xmm0, xmm1
;;      	 b800000000           	mov	eax, 0
;;      	 400f93c0             	setae	al
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3f:	 0f0b                 	ud2	
;;   41:	 0000                 	add	byte ptr [rax], al
;;   43:	 0000                 	add	byte ptr [rax], al
;;   45:	 0000                 	add	byte ptr [rax], al
;;   47:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   4d:	 99                   	cdq	
;;   4e:	 01409a               	add	dword ptr [rax - 0x66], eax
;;   51:	 99                   	cdq	
;;   52:	 99                   	cdq	
;;   53:	 99                   	cdq	
;;   54:	 99                   	cdq	
;;   55:	 99                   	cdq	
;;   56:	 f1                   	int1	
