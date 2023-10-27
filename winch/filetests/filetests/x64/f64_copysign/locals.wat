;;! target = "x86_64"

(module
    (func (result f64)
        (local $foo f64)  
        (local $bar f64)

        (f64.const -1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.copysign
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 4531db               	xor	r11d, r11d
;;    b:	 4c895c2410           	mov	qword ptr [rsp + 0x10], r11
;;   10:	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;   15:	 4c893424             	mov	qword ptr [rsp], r14
;;   19:	 f20f10054f000000     	movsd	xmm0, qword ptr [rip + 0x4f]
;;   21:	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;   27:	 f20f100549000000     	movsd	xmm0, qword ptr [rip + 0x49]
;;   2f:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;   35:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   3b:	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;   41:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   4b:	 664d0f6efb           	movq	xmm15, r11
;;   50:	 66410f54c7           	andpd	xmm0, xmm15
;;   55:	 66440f55f9           	andnpd	xmm15, xmm1
;;   5a:	 66410f28cf           	movapd	xmm1, xmm15
;;   5f:	 660f56c8             	orpd	xmm1, xmm0
;;   63:	 660f28c1             	movapd	xmm0, xmm1
;;   67:	 4883c418             	add	rsp, 0x18
;;   6b:	 5d                   	pop	rbp
;;   6c:	 c3                   	ret	
;;   6d:	 0000                 	add	byte ptr [rax], al
;;   6f:	 009a99999999         	add	byte ptr [rdx - 0x66666667], bl
;;   75:	 99                   	cdq	
;;   76:	 f1                   	int1	
;;   77:	 bf9a999999           	mov	edi, 0x9999999a
;;   7c:	 99                   	cdq	
;;   7d:	 99                   	cdq	
