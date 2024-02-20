;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.lt)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872b000000         	ja	0x46
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10051d000000     	movsd	xmm0, qword ptr [rip + 0x1d]
;;      	 f20f100d1d000000     	movsd	xmm1, qword ptr [rip + 0x1d]
;;      	 660f2ec1             	ucomisd	xmm0, xmm1
;;      	 b800000000           	mov	eax, 0
;;      	 400f97c0             	seta	al
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   46:	 0f0b                 	ud2	
