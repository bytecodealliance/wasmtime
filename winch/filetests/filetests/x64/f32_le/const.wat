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
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8726000000         	ja	0x3e
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10051c000000     	movss	xmm0, dword ptr [rip + 0x1c]
;;      	 f30f100d1c000000     	movss	xmm1, dword ptr [rip + 0x1c]
;;      	 0f2ec1               	ucomiss	xmm0, xmm1
;;      	 b800000000           	mov	eax, 0
;;      	 400f93c0             	setae	al
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3e:	 0f0b                 	ud2	
;;   40:	 cdcc                 	int	0xcc
;;   42:	 0c40                 	or	al, 0x40
;;   44:	 0000                 	add	byte ptr [rax], al
;;   46:	 0000                 	add	byte ptr [rax], al
;;   48:	 cdcc                 	int	0xcc
