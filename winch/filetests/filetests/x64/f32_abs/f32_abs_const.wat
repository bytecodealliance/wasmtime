;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const -1.32)
        (f32.abs)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f30f10051c000000     	movss	xmm0, dword ptr [rip + 0x1c]
;;   14:	 41bbffffff7f         	mov	r11d, 0x7fffffff
;;   1a:	 66450f6efb           	movd	xmm15, r11d
;;   1f:	 410f54c7             	andps	xmm0, xmm15
;;   23:	 4883c408             	add	rsp, 8
;;   27:	 5d                   	pop	rbp
;;   28:	 c3                   	ret	
;;   29:	 0000                 	add	byte ptr [rax], al
;;   2b:	 0000                 	add	byte ptr [rax], al
;;   2d:	 0000                 	add	byte ptr [rax], al
;;   2f:	 00c3                 	add	bl, al
;;   31:	 f5                   	cmc	
;;   32:	 a8bf                 	test	al, 0xbf
