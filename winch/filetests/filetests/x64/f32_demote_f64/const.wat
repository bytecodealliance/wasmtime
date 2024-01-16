;;! target = "x86_64"

(module
    (func (result f32)
        (f64.const 1.0)
        (f32.demote_f64)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10050c000000     	movsd	xmm0, qword ptr [rip + 0xc]
;;      	 f20f5ac0             	cvtsd2ss	xmm0, xmm0
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   1e:	 0000                 	add	byte ptr [rax], al
;;   20:	 0000                 	add	byte ptr [rax], al
;;   22:	 0000                 	add	byte ptr [rax], al
;;   24:	 0000                 	add	byte ptr [rax], al
