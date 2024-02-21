;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo f64)  
        (local $bar f64)

        (f64.const 1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.ne
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8760000000         	ja	0x7e
;;   1e:	 4883ec20             	sub	rsp, 0x20
;;      	 48897c2418           	mov	qword ptr [rsp + 0x18], rdi
;;      	 4889742410           	mov	qword ptr [rsp + 0x10], rsi
;;      	 4531db               	xor	r11d, r11d
;;      	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;      	 4c891c24             	mov	qword ptr [rsp], r11
;;      	 f20f100540000000     	movsd	xmm0, qword ptr [rip + 0x40]
;;      	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;      	 f20f10053a000000     	movsd	xmm0, qword ptr [rip + 0x3a]
;;      	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;      	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;      	 f20f104c2408         	movsd	xmm1, qword ptr [rsp + 8]
;;      	 660f2ec8             	ucomisd	xmm1, xmm0
;;      	 b800000000           	mov	eax, 0
;;      	 400f95c0             	setne	al
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f9ac3             	setp	r11b
;;      	 4c09d8               	or	rax, r11
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   7e:	 0f0b                 	ud2	
