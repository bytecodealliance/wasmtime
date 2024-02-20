;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const 1.32)
        (f32.sqrt)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f871a000000         	ja	0x35
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10050d000000     	movss	xmm0, dword ptr [rip + 0xd]
;;      	 f30f51c0             	sqrtss	xmm0, xmm0
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   35:	 0f0b                 	ud2	
;;   37:	 00c3                 	add	bl, al
;;   39:	 f5                   	cmc	
;;   3a:	 a83f                 	test	al, 0x3f
