;;! target = "x86_64"

(module
    (func (result f64)
        f32.const 1.0
        f64.promote_f32
        block
        end
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f30f10051c000000     	movss	xmm0, dword ptr [rip + 0x1c]
;;   14:	 f30f5ac0             	cvtss2sd	xmm0, xmm0
;;   18:	 4883ec08             	sub	rsp, 8
;;   1c:	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;   21:	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;   26:	 4883c408             	add	rsp, 8
;;   2a:	 4883c408             	add	rsp, 8
;;   2e:	 5d                   	pop	rbp
;;   2f:	 c3                   	ret	
;;   30:	 0000                 	add	byte ptr [rax], al
