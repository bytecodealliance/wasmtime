;;! target = "x86_64"

(module
    (func (result f32)
        i64.const 1
        f32.convert_i64_s
        block
        end
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c001000000       	mov	rax, 1
;;   13:	 f3480f2ac0           	cvtsi2ss	xmm0, rax
;;   18:	 4883ec04             	sub	rsp, 4
;;   1c:	 f30f110424           	movss	dword ptr [rsp], xmm0
;;   21:	 f30f100424           	movss	xmm0, dword ptr [rsp]
;;   26:	 4883c404             	add	rsp, 4
;;   2a:	 4883c408             	add	rsp, 8
;;   2e:	 5d                   	pop	rbp
;;   2f:	 c3                   	ret	
