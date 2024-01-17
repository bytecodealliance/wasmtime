;;! target = "x86_64"

(module
    (func (result i32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.le)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10051c000000     	movss	xmm0, dword ptr [rip + 0x1c]
;;      	 f30f100d1c000000     	movss	xmm1, dword ptr [rip + 0x1c]
;;      	 0f2ec1               	ucomiss	xmm0, xmm1
;;      	 b800000000           	mov	eax, 0
;;      	 400f93c0             	setae	al
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   2e:	 0000                 	add	byte ptr [rax], al
;;   30:	 cdcc                 	int	0xcc
;;   32:	 0c40                 	or	al, 0x40
;;   34:	 0000                 	add	byte ptr [rax], al
;;   36:	 0000                 	add	byte ptr [rax], al
;;   38:	 cdcc                 	int	0xcc
