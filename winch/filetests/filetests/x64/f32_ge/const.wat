;;! target = "x86_64"

(module
    (func (result i32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.ge)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8737000000         	ja	0x52
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10052d000000     	movss	xmm0, dword ptr [rip + 0x2d]
;;      	 f30f100d2d000000     	movss	xmm1, dword ptr [rip + 0x2d]
;;      	 0f2ec8               	ucomiss	xmm1, xmm0
;;      	 b800000000           	mov	eax, 0
;;      	 400f93c0             	setae	al
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f9bc3             	setnp	r11b
;;      	 4421d8               	and	eax, r11d
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   52:	 0f0b                 	ud2	
;;   54:	 0000                 	add	byte ptr [rax], al
;;   56:	 0000                 	add	byte ptr [rax], al
;;   58:	 cdcc                 	int	0xcc
;;   5a:	 0c40                 	or	al, 0x40
;;   5c:	 0000                 	add	byte ptr [rax], al
;;   5e:	 0000                 	add	byte ptr [rax], al
;;   60:	 cdcc                 	int	0xcc
