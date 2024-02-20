;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.mul)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8726000000         	ja	0x41
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10051d000000     	movsd	xmm0, qword ptr [rip + 0x1d]
;;      	 f20f100d1d000000     	movsd	xmm1, qword ptr [rip + 0x1d]
;;      	 f20f59c8             	mulsd	xmm1, xmm0
;;      	 660f28c1             	movapd	xmm0, xmm1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   41:	 0f0b                 	ud2	
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
