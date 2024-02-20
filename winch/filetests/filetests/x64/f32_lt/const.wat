;;! target = "x86_64"

(module
    (func (result i32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.lt)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872a000000         	ja	0x45
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10051d000000     	movss	xmm0, dword ptr [rip + 0x1d]
;;      	 f30f100d1d000000     	movss	xmm1, dword ptr [rip + 0x1d]
;;      	 0f2ec1               	ucomiss	xmm0, xmm1
;;      	 b800000000           	mov	eax, 0
;;      	 400f97c0             	seta	al
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   45:	 0f0b                 	ud2	
;;   47:	 00cd                 	add	ch, cl
;;   49:	 cc                   	int3	
;;   4a:	 0c40                 	or	al, 0x40
;;   4c:	 0000                 	add	byte ptr [rax], al
;;   4e:	 0000                 	add	byte ptr [rax], al
;;   50:	 cdcc                 	int	0xcc
