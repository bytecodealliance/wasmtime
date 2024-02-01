;;! target = "x86_64"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.div)
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
;;      	 f30f100d1c000000     	movss	xmm1, dword ptr [rip + 0x1c]
;;      	 f30f5ec8             	divss	xmm1, xmm0
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   39:	 0f0b                 	ud2	
;;   3b:	 0000                 	add	byte ptr [rax], al
;;   3d:	 0000                 	add	byte ptr [rax], al
;;   3f:	 00cd                 	add	ch, cl
;;   41:	 cc                   	int3	
;;   42:	 0c40                 	or	al, 0x40
;;   44:	 0000                 	add	byte ptr [rax], al
;;   46:	 0000                 	add	byte ptr [rax], al
;;   48:	 cdcc                 	int	0xcc
