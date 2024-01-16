;;! target = "x86_64"

(module
    (func (result i32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.eq)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10052c000000     	movss	xmm0, dword ptr [rip + 0x2c]
;;      	 f30f100d2c000000     	movss	xmm1, dword ptr [rip + 0x2c]
;;      	 0f2ec8               	ucomiss	xmm1, xmm0
;;      	 b800000000           	mov	eax, 0
;;      	 400f94c0             	sete	al
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f9bc3             	setnp	r11b
;;      	 4421d8               	and	eax, r11d
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3b:	 0000                 	add	byte ptr [rax], al
;;   3d:	 0000                 	add	byte ptr [rax], al
;;   3f:	 00cd                 	add	ch, cl
;;   41:	 cc                   	int3	
;;   42:	 0c40                 	or	al, 0x40
;;   44:	 0000                 	add	byte ptr [rax], al
;;   46:	 0000                 	add	byte ptr [rax], al
;;   48:	 cdcc                 	int	0xcc
