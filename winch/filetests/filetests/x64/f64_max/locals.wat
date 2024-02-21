;;! target = "x86_64"

(module
    (func (result f64)
        (local $foo f64)  
        (local $bar f64)

        (f64.const 1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.max
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8771000000         	ja	0x8f
;;   1e:	 4883ec20             	sub	rsp, 0x20
;;      	 48897c2418           	mov	qword ptr [rsp + 0x18], rdi
;;      	 4889742410           	mov	qword ptr [rsp + 0x10], rsi
;;      	 4531db               	xor	r11d, r11d
;;      	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;      	 4c891c24             	mov	qword ptr [rsp], r11
;;      	 f20f100558000000     	movsd	xmm0, qword ptr [rip + 0x58]
;;      	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;      	 f20f100552000000     	movsd	xmm0, qword ptr [rip + 0x52]
;;      	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;      	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;      	 f20f104c2408         	movsd	xmm1, qword ptr [rsp + 8]
;;      	 660f2ec8             	ucomisd	xmm1, xmm0
;;      	 0f8519000000         	jne	0x81
;;      	 0f8a09000000         	jp	0x77
;;   6e:	 660f54c8             	andpd	xmm1, xmm0
;;      	 e90e000000           	jmp	0x85
;;   77:	 f20f58c8             	addsd	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x85
;;   81:	 f20f5fc8             	maxsd	xmm1, xmm0
;;      	 660f28c1             	movapd	xmm0, xmm1
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   8f:	 0f0b                 	ud2	
;;   91:	 0000                 	add	byte ptr [rax], al
;;   93:	 0000                 	add	byte ptr [rax], al
;;   95:	 0000                 	add	byte ptr [rax], al
;;   97:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   9d:	 99                   	cdq	
;;   9e:	 f1                   	int1	
