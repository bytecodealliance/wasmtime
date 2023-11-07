;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo f64)  
        (local $bar f64)

        (f64.const 1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.le
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 4531db               	xor	r11d, r11d
;;    b:	 4c895c2410           	mov	qword ptr [rsp + 0x10], r11
;;   10:	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;   15:	 4c893424             	mov	qword ptr [rsp], r14
;;   19:	 f20f100537000000     	movsd	xmm0, qword ptr [rip + 0x37]
;;   21:	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;   27:	 f20f100531000000     	movsd	xmm0, qword ptr [rip + 0x31]
;;   2f:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;   35:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   3b:	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;   41:	 660f2ec1             	ucomisd	xmm0, xmm1
;;   45:	 b800000000           	mov	eax, 0
;;   4a:	 400f93c0             	setae	al
;;   4e:	 4883c418             	add	rsp, 0x18
;;   52:	 5d                   	pop	rbp
;;   53:	 c3                   	ret	
;;   54:	 0000                 	add	byte ptr [rax], al
;;   56:	 0000                 	add	byte ptr [rax], al
