;;! target = "x86_64"

(module
    (func (result f32)
        f64.const 1.0
        f32.demote_f64
        block
        end
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f10051c000000     	movsd	xmm0, qword ptr [rip + 0x1c]
;;   14:	 f20f5ac0             	cvtsd2ss	xmm0, xmm0
;;   18:	 4883ec04             	sub	rsp, 4
;;   1c:	 f30f110424           	movss	dword ptr [rsp], xmm0
;;   21:	 f30f100424           	movss	xmm0, dword ptr [rsp]
;;   26:	 4883c404             	add	rsp, 4
;;   2a:	 4883c408             	add	rsp, 8
;;   2e:	 5d                   	pop	rbp
;;   2f:	 c3                   	ret	
;;   30:	 0000                 	add	byte ptr [rax], al
;;   32:	 0000                 	add	byte ptr [rax], al
;;   34:	 0000                 	add	byte ptr [rax], al
