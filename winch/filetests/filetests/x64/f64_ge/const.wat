;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.ge)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10052c000000     	movsd	xmm0, qword ptr [rip + 0x2c]
;;      	 f20f100d2c000000     	movsd	xmm1, qword ptr [rip + 0x2c]
;;      	 660f2ec8             	ucomisd	xmm1, xmm0
;;      	 b800000000           	mov	eax, 0
;;      	 400f93c0             	setae	al
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f9bc3             	setnp	r11b
;;      	 4c21d8               	and	rax, r11
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3c:	 0000                 	add	byte ptr [rax], al
;;   3e:	 0000                 	add	byte ptr [rax], al
