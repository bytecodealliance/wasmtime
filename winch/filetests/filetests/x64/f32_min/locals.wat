;;! target = "x86_64"

(module
    (func (result f32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const 1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.min
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8763000000         	ja	0x7b
;;   18:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f100553000000     	movss	xmm0, dword ptr [rip + 0x53]
;;      	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;      	 f30f10054d000000     	movss	xmm0, dword ptr [rip + 0x4d]
;;      	 f30f11442408         	movss	dword ptr [rsp + 8], xmm0
;;      	 f30f10442408         	movss	xmm0, dword ptr [rsp + 8]
;;      	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;      	 0f2ec8               	ucomiss	xmm1, xmm0
;;      	 0f8518000000         	jne	0x6e
;;      	 0f8a08000000         	jp	0x64
;;   5c:	 0f56c8               	orps	xmm1, xmm0
;;      	 e90e000000           	jmp	0x72
;;   64:	 f30f58c8             	addss	xmm1, xmm0
;;      	 0f8a04000000         	jp	0x72
;;   6e:	 f30f5dc8             	minss	xmm1, xmm0
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   7b:	 0f0b                 	ud2	
;;   7d:	 0000                 	add	byte ptr [rax], al
;;   7f:	 00cd                 	add	ch, cl
;;   81:	 cc                   	int3	
