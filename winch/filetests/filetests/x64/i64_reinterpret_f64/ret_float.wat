;;! target = "x86_64"

(module
    (func (result f64)
        f64.const 1.0
        i64.reinterpret_f64
        drop
        f64.const 1.0
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f100514000000     	movsd	xmm0, qword ptr [rip + 0x14]
;;   14:	 66480f7ec0           	movq	rax, xmm0
;;   19:	 f20f100507000000     	movsd	xmm0, qword ptr [rip + 7]
;;   21:	 4883c408             	add	rsp, 8
;;   25:	 5d                   	pop	rbp
;;   26:	 c3                   	ret	
;;   27:	 0000                 	add	byte ptr [rax], al
;;   29:	 0000                 	add	byte ptr [rax], al
;;   2b:	 0000                 	add	byte ptr [rax], al
;;   2d:	 00f0                 	add	al, dh
