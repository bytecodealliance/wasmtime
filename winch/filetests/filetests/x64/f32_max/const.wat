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
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8746000000         	ja	0x61
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10053d000000     	movss	xmm0, dword ptr [rip + 0x3d]
;;      	 f30f100d3d000000     	movss	xmm1, dword ptr [rip + 0x3d]
;;      	 0f2ec8               	ucomiss	xmm1, xmm0
;;      	 0f8518000000         	jne	0x54
;;      	 0f8a08000000         	jp	0x4a
;;   42:	 0f54c8               	andps	xmm1, xmm0
;;      	 e90e000000           	jmp	0x58
;;   4a:	 f30f58c8             	addss	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x58
;;   54:	 f30f5fc8             	maxss	xmm1, xmm0
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   61:	 0f0b                 	ud2	
;;   63:	 0000                 	add	byte ptr [rax], al
;;   65:	 0000                 	add	byte ptr [rax], al
;;   67:	 00cd                 	add	ch, cl
;;   69:	 cc                   	int3	
;;   6a:	 0c40                 	or	al, 0x40
;;   6c:	 0000                 	add	byte ptr [rax], al
;;   6e:	 0000                 	add	byte ptr [rax], al
;;   70:	 cdcc                 	int	0xcc
