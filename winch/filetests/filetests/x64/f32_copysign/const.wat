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
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8737000000         	ja	0x4f
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f100534000000     	movss	xmm0, dword ptr [rip + 0x34]
;;      	 f30f100d34000000     	movss	xmm1, dword ptr [rip + 0x34]
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
;;   4f:	 0f0b                 	ud2	
;;   51:	 0000                 	add	byte ptr [rax], al
;;   53:	 0000                 	add	byte ptr [rax], al
;;   55:	 0000                 	add	byte ptr [rax], al
;;   57:	 00cd                 	add	ch, cl
;;   59:	 cc                   	int3	
;;   5a:	 0c40                 	or	al, 0x40
;;   5c:	 0000                 	add	byte ptr [rax], al
;;   5e:	 0000                 	add	byte ptr [rax], al
;;   60:	 cdcc                 	int	0xcc
