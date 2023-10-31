;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.mul)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f10051c000000     	movsd	xmm0, qword ptr [rip + 0x1c]
;;   14:	 f20f100d1c000000     	movsd	xmm1, qword ptr [rip + 0x1c]
;;   1c:	 f20f59c8             	mulsd	xmm1, xmm0
;;   20:	 660f28c1             	movapd	xmm0, xmm1
;;   24:	 4883c408             	add	rsp, 8
;;   28:	 5d                   	pop	rbp
;;   29:	 c3                   	ret	
;;   2a:	 0000                 	add	byte ptr [rax], al
;;   2c:	 0000                 	add	byte ptr [rax], al
;;   2e:	 0000                 	add	byte ptr [rax], al
