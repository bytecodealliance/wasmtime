;;! target = "x86_64"

(module
    (func (result i64)
        (local $foo i64)  
        (local $bar i64)

        (i64.const 10)
        (local.set $foo)

        (i64.const 20)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        i64.mul
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8747000000         	ja	0x65
;;   1e:	 4883ec20             	sub	rsp, 0x20
;;      	 48897c2418           	mov	qword ptr [rsp + 0x18], rdi
;;      	 4889742410           	mov	qword ptr [rsp + 0x10], rsi
;;      	 4531db               	xor	r11d, r11d
;;      	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;      	 4c891c24             	mov	qword ptr [rsp], r11
;;      	 48c7c00a000000       	mov	rax, 0xa
;;      	 4889442408           	mov	qword ptr [rsp + 8], rax
;;      	 48c7c014000000       	mov	rax, 0x14
;;      	 48890424             	mov	qword ptr [rsp], rax
;;      	 488b0424             	mov	rax, qword ptr [rsp]
;;      	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;      	 480fafc8             	imul	rcx, rax
;;      	 4889c8               	mov	rax, rcx
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   65:	 0f0b                 	ud2	
