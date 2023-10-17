;;! target = "x86_64"

(module
    (func (param f64) (param f64) (result i32)
        (local.get 0)
        (local.get 1)
        (f64.ne)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;    e:	 f20f114c2408         	movsd	qword ptr [rsp + 8], xmm1
;;   14:	 4c893424             	mov	qword ptr [rsp], r14
;;   18:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   1e:	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;   24:	 660f2ec8             	ucomisd	xmm1, xmm0
;;   28:	 b800000000           	mov	eax, 0
;;   2d:	 400f95c0             	setne	al
;;   31:	 41bb00000000         	mov	r11d, 0
;;   37:	 410f9ac3             	setp	r11b
;;   3b:	 4c09d8               	or	rax, r11
;;   3e:	 4883c418             	add	rsp, 0x18
;;   42:	 5d                   	pop	rbp
;;   43:	 c3                   	ret	
