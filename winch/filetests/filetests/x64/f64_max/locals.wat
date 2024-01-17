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
;;      	 4883ec18             	sub	rsp, 0x18
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f876a000000         	ja	0x82
;;   18:	 4531db               	xor	r11d, r11d
;;      	 4c895c2410           	mov	qword ptr [rsp + 0x10], r11
;;      	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f100557000000     	movsd	xmm0, qword ptr [rip + 0x57]
;;      	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;      	 f20f100551000000     	movsd	xmm0, qword ptr [rip + 0x51]
;;      	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;      	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;      	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;      	 660f2ec8             	ucomisd	xmm1, xmm0
;;      	 0f8519000000         	jne	0x74
;;      	 0f8a09000000         	jp	0x6a
;;   61:	 660f54c8             	andpd	xmm1, xmm0
;;      	 e90e000000           	jmp	0x78
;;   6a:	 f20f58c8             	addsd	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x78
;;   74:	 f20f5fc8             	maxsd	xmm1, xmm0
;;      	 660f28c1             	movapd	xmm0, xmm1
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   82:	 0f0b                 	ud2	
;;   84:	 0000                 	add	byte ptr [rax], al
;;   86:	 0000                 	add	byte ptr [rax], al
