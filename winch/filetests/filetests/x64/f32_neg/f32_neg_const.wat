;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const -1.32)
        (f32.neg)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10051c000000     	movss	xmm0, dword ptr [rip + 0x1c]
;;      	 41bb00000080         	mov	r11d, 0x80000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f57c7             	xorps	xmm0, xmm15
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   29:	 0000                 	add	byte ptr [rax], al
;;   2b:	 0000                 	add	byte ptr [rax], al
;;   2d:	 0000                 	add	byte ptr [rax], al
;;   2f:	 00c3                 	add	bl, al
;;   31:	 f5                   	cmc	
;;   32:	 a8bf                 	test	al, 0xbf
