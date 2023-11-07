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
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f30f100543000000     	movss	xmm0, dword ptr [rip + 0x43]
;;   1d:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;   23:	 f30f10053d000000     	movss	xmm0, dword ptr [rip + 0x3d]
;;   2b:	 f30f11442408         	movss	dword ptr [rsp + 8], xmm0
;;   31:	 f30f10442408         	movss	xmm0, dword ptr [rsp + 8]
;;   37:	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;   3d:	 0f2ec8               	ucomiss	xmm1, xmm0
;;   40:	 b800000000           	mov	eax, 0
;;   45:	 400f95c0             	setne	al
;;   49:	 41bb00000000         	mov	r11d, 0
;;   4f:	 410f9ac3             	setp	r11b
;;   53:	 4409d8               	or	eax, r11d
;;   56:	 4883c410             	add	rsp, 0x10
;;   5a:	 5d                   	pop	rbp
;;   5b:	 c3                   	ret	
;;   5c:	 0000                 	add	byte ptr [rax], al
;;   5e:	 0000                 	add	byte ptr [rax], al
;;   60:	 cdcc                 	int	0xcc
