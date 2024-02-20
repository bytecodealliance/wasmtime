;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const -1.32)
        (f32.abs)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8725000000         	ja	0x40
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10051d000000     	movss	xmm0, dword ptr [rip + 0x1d]
;;      	 41bbffffff7f         	mov	r11d, 0x7fffffff
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f54c7             	andps	xmm0, xmm15
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   40:	 0f0b                 	ud2	
;;   42:	 0000                 	add	byte ptr [rax], al
;;   44:	 0000                 	add	byte ptr [rax], al
;;   46:	 0000                 	add	byte ptr [rax], al
;;   48:	 c3                   	ret	
;;   49:	 f5                   	cmc	
;;   4a:	 a8bf                 	test	al, 0xbf
