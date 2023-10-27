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
        f64.ne
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 4531db               	xor	r11d, r11d
;;    b:	 4c895c2410           	mov	qword ptr [rsp + 0x10], r11
;;   10:	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;   15:	 4c893424             	mov	qword ptr [rsp], r14
;;   19:	 f20f100547000000     	movsd	xmm0, qword ptr [rip + 0x47]
;;   21:	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;   27:	 f20f100541000000     	movsd	xmm0, qword ptr [rip + 0x41]
;;   2f:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;   35:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   3b:	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;   41:	 660f2ec8             	ucomisd	xmm1, xmm0
;;   45:	 b800000000           	mov	eax, 0
;;   4a:	 400f95c0             	setne	al
;;   4e:	 41bb00000000         	mov	r11d, 0
;;   54:	 410f9ac3             	setp	r11b
;;   58:	 4c09d8               	or	rax, r11
;;   5b:	 4883c418             	add	rsp, 0x18
;;   5f:	 5d                   	pop	rbp
;;   60:	 c3                   	ret	
;;   61:	 0000                 	add	byte ptr [rax], al
;;   63:	 0000                 	add	byte ptr [rax], al
;;   65:	 0000                 	add	byte ptr [rax], al
;;   67:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   6d:	 99                   	cdq	
;;   6e:	 f1                   	int1	
