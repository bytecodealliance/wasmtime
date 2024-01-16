;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const -1.1)
        (f32.const 2.2)
        (f32.copysign)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10052c000000     	movss	xmm0, dword ptr [rip + 0x2c]
;;      	 f30f100d2c000000     	movss	xmm1, dword ptr [rip + 0x2c]
;;      	 41bb00000080         	mov	r11d, 0x80000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f54c7             	andps	xmm0, xmm15
;;      	 440f55f9             	andnps	xmm15, xmm1
;;      	 410f28cf             	movaps	xmm1, xmm15
;;      	 0f56c8               	orps	xmm1, xmm0
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3f:	 00cd                 	add	ch, cl
;;   41:	 cc                   	int3	
;;   42:	 0c40                 	or	al, 0x40
;;   44:	 0000                 	add	byte ptr [rax], al
;;   46:	 0000                 	add	byte ptr [rax], al
;;   48:	 cdcc                 	int	0xcc
