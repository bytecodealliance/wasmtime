;;! target = "x86_64"

(module
    (func (result f64)
        (local $foo f64)  
        (local $bar f64)

        (f64.const -1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.copysign
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec18             	sub	rsp, 0x18
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8765000000         	ja	0x7d
;;   18:	 4531db               	xor	r11d, r11d
;;      	 4c895c2410           	mov	qword ptr [rsp + 0x10], r11
;;      	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10054f000000     	movsd	xmm0, qword ptr [rip + 0x4f]
;;      	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;      	 f20f100549000000     	movsd	xmm0, qword ptr [rip + 0x49]
;;      	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;      	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;      	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;      	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f54c7           	andpd	xmm0, xmm15
;;      	 66440f55f9           	andnpd	xmm15, xmm1
;;      	 66410f28cf           	movapd	xmm1, xmm15
;;      	 660f56c8             	orpd	xmm1, xmm0
;;      	 660f28c1             	movapd	xmm0, xmm1
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   7d:	 0f0b                 	ud2	
;;   7f:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   85:	 99                   	cdq	
;;   86:	 f1                   	int1	
;;   87:	 bf9a999999           	mov	edi, 0x9999999a
;;   8c:	 99                   	cdq	
;;   8d:	 99                   	cdq	
