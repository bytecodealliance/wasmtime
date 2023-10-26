;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.eq)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f10052c000000     	movsd	xmm0, qword ptr [rip + 0x2c]
;;   14:	 f20f100d2c000000     	movsd	xmm1, qword ptr [rip + 0x2c]
;;   1c:	 660f2ec8             	ucomisd	xmm1, xmm0
;;   20:	 b800000000           	mov	eax, 0
;;   25:	 400f94c0             	sete	al
;;   29:	 41bb00000000         	mov	r11d, 0
;;   2f:	 410f9bc3             	setnp	r11b
;;   33:	 4c21d8               	and	rax, r11
;;   36:	 4883c408             	add	rsp, 8
;;   3a:	 5d                   	pop	rbp
;;   3b:	 c3                   	ret	
;;   3c:	 0000                 	add	byte ptr [rax], al
;;   3e:	 0000                 	add	byte ptr [rax], al
