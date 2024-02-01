;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.max)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8742000000         	ja	0x5a
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10053c000000     	movss	xmm0, dword ptr [rip + 0x3c]
;;      	 f30f100d3c000000     	movss	xmm1, dword ptr [rip + 0x3c]
;;      	 0f2ec8               	ucomiss	xmm1, xmm0
;;      	 0f8518000000         	jne	0x4d
;;      	 0f8a08000000         	jp	0x43
;;   3b:	 0f54c8               	andps	xmm1, xmm0
;;      	 e90e000000           	jmp	0x51
;;   43:	 f30f58c8             	addss	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x51
;;   4d:	 f30f5fc8             	maxss	xmm1, xmm0
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5a:	 0f0b                 	ud2	
;;   5c:	 0000                 	add	byte ptr [rax], al
;;   5e:	 0000                 	add	byte ptr [rax], al
;;   60:	 cdcc                 	int	0xcc
;;   62:	 0c40                 	or	al, 0x40
;;   64:	 0000                 	add	byte ptr [rax], al
;;   66:	 0000                 	add	byte ptr [rax], al
;;   68:	 cdcc                 	int	0xcc
