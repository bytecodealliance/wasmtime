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
        f64.min
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 4531db               	xor	r11d, r11d
;;    b:	 4c895c2410           	mov	qword ptr [rsp + 0x10], r11
;;   10:	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;   15:	 4c893424             	mov	qword ptr [rsp], r14
;;   19:	 f20f100557000000     	movsd	xmm0, qword ptr [rip + 0x57]
;;   21:	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;   27:	 f20f100551000000     	movsd	xmm0, qword ptr [rip + 0x51]
;;   2f:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;   35:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   3b:	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;   41:	 660f2ec8             	ucomisd	xmm1, xmm0
;;   45:	 0f8519000000         	jne	0x64
;;   4b:	 0f8a09000000         	jp	0x5a
;;   51:	 660f56c8             	orpd	xmm1, xmm0
;;   55:	 e90e000000           	jmp	0x68
;;   5a:	 f20f58c8             	addsd	xmm1, xmm0
;;   5e:	 0f8a04000000         	jp	0x68
;;   64:	 f20f5dc8             	minsd	xmm1, xmm0
;;   68:	 660f28c1             	movapd	xmm0, xmm1
;;   6c:	 4883c418             	add	rsp, 0x18
;;   70:	 5d                   	pop	rbp
;;   71:	 c3                   	ret	
;;   72:	 0000                 	add	byte ptr [rax], al
;;   74:	 0000                 	add	byte ptr [rax], al
;;   76:	 0000                 	add	byte ptr [rax], al
