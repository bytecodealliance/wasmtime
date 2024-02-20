;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const 1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.ne
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8758000000         	ja	0x73
;;   1b:	 4883ec10             	sub	rsp, 0x10
;;      	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f100544000000     	movss	xmm0, dword ptr [rip + 0x44]
;;      	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;      	 f30f10053e000000     	movss	xmm0, dword ptr [rip + 0x3e]
;;      	 f30f11442408         	movss	dword ptr [rsp + 8], xmm0
;;      	 f30f10442408         	movss	xmm0, dword ptr [rsp + 8]
;;      	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;      	 0f2ec8               	ucomiss	xmm1, xmm0
;;      	 b800000000           	mov	eax, 0
;;      	 400f95c0             	setne	al
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f9ac3             	setp	r11b
;;      	 4409d8               	or	eax, r11d
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   73:	 0f0b                 	ud2	
;;   75:	 0000                 	add	byte ptr [rax], al
;;   77:	 00cd                 	add	ch, cl
;;   79:	 cc                   	int3	
