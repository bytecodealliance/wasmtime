;;! target = "x86_64"
;;! flags = ["has_sse41"]

(module
    (func (result f32)
        (f32.const -1.32)
        (f32.floor)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8721000000         	ja	0x3f
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 f30f100515000000     	movss	xmm0, dword ptr [rip + 0x15]
;;      	 660f3a0ac001         	roundss	xmm0, xmm0, 1
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3f:	 0f0b                 	ud2	
;;   41:	 0000                 	add	byte ptr [rax], al
;;   43:	 0000                 	add	byte ptr [rax], al
;;   45:	 0000                 	add	byte ptr [rax], al
;;   47:	 00c3                 	add	bl, al
;;   49:	 f5                   	cmc	
;;   4a:	 a8bf                 	test	al, 0xbf
