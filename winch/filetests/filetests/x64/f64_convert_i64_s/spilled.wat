;;! target = "x86_64"

(module
    (func (result f64)
        i64.const 1
        f64.convert_i64_s
        block
        end
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c001000000       	mov	rax, 1
;;   13:	 f2480f2ac0           	cvtsi2sd	xmm0, rax
;;   18:	 4883ec08             	sub	rsp, 8
;;   1c:	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;   21:	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;   26:	 4883c408             	add	rsp, 8
;;   2a:	 4883c408             	add	rsp, 8
;;   2e:	 5d                   	pop	rbp
;;   2f:	 c3                   	ret	
