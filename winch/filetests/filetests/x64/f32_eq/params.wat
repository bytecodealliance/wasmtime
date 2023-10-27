;;! target = "x86_64"

(module
    (func (param f32) (param f32) (result i32)
        (local.get 0)
        (local.get 1)
        (f32.eq)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;    e:	 f30f114c2408         	movss	dword ptr [rsp + 8], xmm1
;;   14:	 4c893424             	mov	qword ptr [rsp], r14
;;   18:	 f30f10442408         	movss	xmm0, dword ptr [rsp + 8]
;;   1e:	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;   24:	 0f2ec8               	ucomiss	xmm1, xmm0
;;   27:	 b800000000           	mov	eax, 0
;;   2c:	 400f94c0             	sete	al
;;   30:	 41bb00000000         	mov	r11d, 0
;;   36:	 410f9bc3             	setnp	r11b
;;   3a:	 4421d8               	and	eax, r11d
;;   3d:	 4883c410             	add	rsp, 0x10
;;   41:	 5d                   	pop	rbp
;;   42:	 c3                   	ret	
