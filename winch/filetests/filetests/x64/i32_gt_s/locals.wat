;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo i32)
        (local $bar i32)

        (i32.const 2)
        (local.set $foo)
        (i32.const 3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i32.gt_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8741000000         	ja	0x5f
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 48c7042400000000     	mov	qword ptr [rsp], 0
;;      	 b802000000           	mov	eax, 2
;;      	 89442404             	mov	dword ptr [rsp + 4], eax
;;      	 b803000000           	mov	eax, 3
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 8b0424               	mov	eax, dword ptr [rsp]
;;      	 8b4c2404             	mov	ecx, dword ptr [rsp + 4]
;;      	 39c1                 	cmp	ecx, eax
;;      	 b900000000           	mov	ecx, 0
;;      	 400f9fc1             	setg	cl
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5f:	 0f0b                 	ud2	
