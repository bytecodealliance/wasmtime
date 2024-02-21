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
        f64.lt
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8753000000         	ja	0x71
;;   1e:	 4883ec20             	sub	rsp, 0x20
;;      	 48897c2418           	mov	qword ptr [rsp + 0x18], rdi
;;      	 4889742410           	mov	qword ptr [rsp + 0x10], rsi
;;      	 4531db               	xor	r11d, r11d
;;      	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;      	 4c891c24             	mov	qword ptr [rsp], r11
;;      	 f20f100538000000     	movsd	xmm0, qword ptr [rip + 0x38]
;;      	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;      	 f20f100532000000     	movsd	xmm0, qword ptr [rip + 0x32]
;;      	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;      	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;      	 f20f104c2408         	movsd	xmm1, qword ptr [rsp + 8]
;;      	 660f2ec1             	ucomisd	xmm0, xmm1
;;      	 b800000000           	mov	eax, 0
;;      	 400f97c0             	seta	al
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   71:	 0f0b                 	ud2	
;;   73:	 0000                 	add	byte ptr [rax], al
;;   75:	 0000                 	add	byte ptr [rax], al
;;   77:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   7d:	 99                   	cdq	
;;   7e:	 f1                   	int1	
