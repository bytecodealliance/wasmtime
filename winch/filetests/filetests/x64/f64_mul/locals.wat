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
        f64.mul
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f874b000000         	ja	0x66
;;   1b:	 4883ec18             	sub	rsp, 0x18
;;      	 4531db               	xor	r11d, r11d
;;      	 4c895c2410           	mov	qword ptr [rsp + 0x10], r11
;;      	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f100530000000     	movsd	xmm0, qword ptr [rip + 0x30]
;;      	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;      	 f20f10052a000000     	movsd	xmm0, qword ptr [rip + 0x2a]
;;      	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;      	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;      	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;      	 f20f59c8             	mulsd	xmm1, xmm0
;;      	 660f28c1             	movapd	xmm0, xmm1
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   66:	 0f0b                 	ud2	
