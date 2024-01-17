;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const -1.32)
        (f32.abs)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8721000000         	ja	0x39
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10051c000000     	movss	xmm0, dword ptr [rip + 0x1c]
;;      	 41bbffffff7f         	mov	r11d, 0x7fffffff
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f54c7             	andps	xmm0, xmm15
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   39:	 0f0b                 	ud2	
;;   3b:	 0000                 	add	byte ptr [rax], al
;;   3d:	 0000                 	add	byte ptr [rax], al
;;   3f:	 00c3                 	add	bl, al
;;   41:	 f5                   	cmc	
;;   42:	 a8bf                 	test	al, 0xbf
