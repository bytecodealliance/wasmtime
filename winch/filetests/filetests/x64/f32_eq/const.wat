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
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8733000000         	ja	0x4b
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
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
;;   4b:	 0f0b                 	ud2	
;;   4d:	 0000                 	add	byte ptr [rax], al
;;   4f:	 00cd                 	add	ch, cl
;;   51:	 cc                   	int3	
;;   52:	 0c40                 	or	al, 0x40
;;   54:	 0000                 	add	byte ptr [rax], al
;;   56:	 0000                 	add	byte ptr [rax], al
;;   58:	 cdcc                 	int	0xcc
