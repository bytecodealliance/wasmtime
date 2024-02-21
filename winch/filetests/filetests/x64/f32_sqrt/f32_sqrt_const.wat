;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const 1.32)
        (f32.sqrt)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f871f000000         	ja	0x3d
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 f30f10050d000000     	movss	xmm0, dword ptr [rip + 0xd]
;;      	 f30f51c0             	sqrtss	xmm0, xmm0
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3d:	 0f0b                 	ud2	
;;   3f:	 00c3                 	add	bl, al
;;   41:	 f5                   	cmc	
;;   42:	 a83f                 	test	al, 0x3f
