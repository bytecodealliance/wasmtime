;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.lt)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f10051c000000     	movsd	xmm0, qword ptr [rip + 0x1c]
;;   14:	 f20f100d1c000000     	movsd	xmm1, qword ptr [rip + 0x1c]
;;   1c:	 660f2ec1             	ucomisd	xmm0, xmm1
;;   20:	 b800000000           	mov	eax, 0
;;   25:	 400f97c0             	seta	al
;;   29:	 4883c408             	add	rsp, 8
;;   2d:	 5d                   	pop	rbp
;;   2e:	 c3                   	ret	
;;   2f:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   35:	 99                   	cdq	
;;   36:	 01409a               	add	dword ptr [rax - 0x66], eax
;;   39:	 99                   	cdq	
;;   3a:	 99                   	cdq	
;;   3b:	 99                   	cdq	
;;   3c:	 99                   	cdq	
;;   3d:	 99                   	cdq	
;;   3e:	 f1                   	int1	
