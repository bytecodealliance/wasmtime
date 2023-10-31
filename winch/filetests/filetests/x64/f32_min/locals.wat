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
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f30f100553000000     	movss	xmm0, dword ptr [rip + 0x53]
;;   1d:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;   23:	 f30f10054d000000     	movss	xmm0, dword ptr [rip + 0x4d]
;;   2b:	 f30f11442408         	movss	dword ptr [rsp + 8], xmm0
;;   31:	 f30f10442408         	movss	xmm0, dword ptr [rsp + 8]
;;   37:	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;   3d:	 0f2ec8               	ucomiss	xmm1, xmm0
;;   40:	 0f8518000000         	jne	0x5e
;;   46:	 0f8a08000000         	jp	0x54
;;   4c:	 0f56c8               	orps	xmm1, xmm0
;;   4f:	 e90e000000           	jmp	0x62
;;   54:	 f30f58c8             	addss	xmm1, xmm0
;;   58:	 0f8a04000000         	jp	0x62
;;   5e:	 f30f5dc8             	minss	xmm1, xmm0
;;   62:	 0f28c1               	movaps	xmm0, xmm1
;;   65:	 4883c410             	add	rsp, 0x10
;;   69:	 5d                   	pop	rbp
;;   6a:	 c3                   	ret	
;;   6b:	 0000                 	add	byte ptr [rax], al
;;   6d:	 0000                 	add	byte ptr [rax], al
;;   6f:	 00cd                 	add	ch, cl
;;   71:	 cc                   	int3	
