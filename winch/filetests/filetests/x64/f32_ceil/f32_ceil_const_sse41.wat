;;! target = "x86_64"
;;! flags = ["has_sse41"]

(module
    (func (result f32)
        (f32.const -1.32)
        (f32.ceil)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8718000000         	ja	0x30
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f100514000000     	movss	xmm0, dword ptr [rip + 0x14]
;;      	 660f3a0ac002         	roundss	xmm0, xmm0, 2
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   30:	 0f0b                 	ud2	
;;   32:	 0000                 	add	byte ptr [rax], al
;;   34:	 0000                 	add	byte ptr [rax], al
;;   36:	 0000                 	add	byte ptr [rax], al
;;   38:	 c3                   	ret	
;;   39:	 f5                   	cmc	
;;   3a:	 a8bf                 	test	al, 0xbf
