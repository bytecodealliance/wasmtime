;;! target = "x86_64"

(module
    (func (result f64)
        (local $foo f64)  
        (local $bar f64)

        (f64.const 1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.add
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 4531db               	xor	r11d, r11d
;;    b:	 4c895c2410           	mov	qword ptr [rsp + 0x10], r11
;;   10:	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;   15:	 4c893424             	mov	qword ptr [rsp], r14
;;   19:	 f20f10052f000000     	movsd	xmm0, qword ptr [rip + 0x2f]
;;   21:	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;   27:	 f20f100529000000     	movsd	xmm0, qword ptr [rip + 0x29]
;;   2f:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;   35:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   3b:	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;   41:	 f20f58c8             	addsd	xmm1, xmm0
;;   45:	 660f28c1             	movapd	xmm0, xmm1
;;   49:	 4883c418             	add	rsp, 0x18
;;   4d:	 5d                   	pop	rbp
;;   4e:	 c3                   	ret	
;;   4f:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   55:	 99                   	cdq	
;;   56:	 f1                   	int1	
