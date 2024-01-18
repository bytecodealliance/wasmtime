;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.div)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8722000000         	ja	0x3a
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10051c000000     	movsd	xmm0, qword ptr [rip + 0x1c]
;;      	 f20f100d1c000000     	movsd	xmm1, qword ptr [rip + 0x1c]
;;      	 f20f5ec8             	divsd	xmm1, xmm0
;;      	 660f28c1             	movapd	xmm0, xmm1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3a:	 0f0b                 	ud2	
;;   3c:	 0000                 	add	byte ptr [rax], al
;;   3e:	 0000                 	add	byte ptr [rax], al
