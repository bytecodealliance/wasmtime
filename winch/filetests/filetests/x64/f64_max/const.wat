;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.max)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8749000000         	ja	0x64
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10053d000000     	movsd	xmm0, qword ptr [rip + 0x3d]
;;      	 f20f100d3d000000     	movsd	xmm1, qword ptr [rip + 0x3d]
;;      	 660f2ec8             	ucomisd	xmm1, xmm0
;;      	 0f8519000000         	jne	0x56
;;      	 0f8a09000000         	jp	0x4c
;;   43:	 660f54c8             	andpd	xmm1, xmm0
;;      	 e90e000000           	jmp	0x5a
;;   4c:	 f20f58c8             	addsd	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x5a
;;   56:	 f20f5fc8             	maxsd	xmm1, xmm0
;;      	 660f28c1             	movapd	xmm0, xmm1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   64:	 0f0b                 	ud2	
;;   66:	 0000                 	add	byte ptr [rax], al
