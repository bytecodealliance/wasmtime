;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.max)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f30f10053c000000     	movss	xmm0, dword ptr [rip + 0x3c]
;;   14:	 f30f100d3c000000     	movss	xmm1, dword ptr [rip + 0x3c]
;;   1c:	 0f2ec8               	ucomiss	xmm1, xmm0
;;   1f:	 0f8518000000         	jne	0x3d
;;   25:	 0f8a08000000         	jp	0x33
;;   2b:	 0f54c8               	andps	xmm1, xmm0
;;   2e:	 e90e000000           	jmp	0x41
;;   33:	 f30f58c8             	addss	xmm1, xmm0
;;   37:	 0f8a04000000         	jp	0x41
;;   3d:	 f30f5fc8             	maxss	xmm1, xmm0
;;   41:	 0f28c1               	movaps	xmm0, xmm1
;;   44:	 4883c408             	add	rsp, 8
;;   48:	 5d                   	pop	rbp
;;   49:	 c3                   	ret	
;;   4a:	 0000                 	add	byte ptr [rax], al
;;   4c:	 0000                 	add	byte ptr [rax], al
;;   4e:	 0000                 	add	byte ptr [rax], al
;;   50:	 cdcc                 	int	0xcc
;;   52:	 0c40                 	or	al, 0x40
;;   54:	 0000                 	add	byte ptr [rax], al
;;   56:	 0000                 	add	byte ptr [rax], al
;;   58:	 cdcc                 	int	0xcc
