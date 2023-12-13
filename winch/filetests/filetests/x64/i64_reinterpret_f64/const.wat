;;! target = "x86_64"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.reinterpret_f64)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f10050c000000     	movsd	xmm0, qword ptr [rip + 0xc]
;;   14:	 66480f7ec0           	movq	rax, xmm0
;;   19:	 4883c408             	add	rsp, 8
;;   1d:	 5d                   	pop	rbp
;;   1e:	 c3                   	ret	
;;   1f:	 0000                 	add	byte ptr [rax], al
;;   21:	 0000                 	add	byte ptr [rax], al
;;   23:	 0000                 	add	byte ptr [rax], al
;;   25:	 00f0                 	add	al, dh
