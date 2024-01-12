;;! target = "x86_64"

(module
    (func (result f64)
        i64.const 1
        f64.convert_i64_u
        block
        end
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c101000000       	mov	rcx, 1
;;   13:	 4883f900             	cmp	rcx, 0
;;   17:	 0f8c0a000000         	jl	0x27
;;   1d:	 f2480f2ac1           	cvtsi2sd	xmm0, rcx
;;   22:	 e91a000000           	jmp	0x41
;;   27:	 4989cb               	mov	r11, rcx
;;   2a:	 49c1eb01             	shr	r11, 1
;;   2e:	 4889c8               	mov	rax, rcx
;;   31:	 4883e001             	and	rax, 1
;;   35:	 4c09d8               	or	rax, r11
;;   38:	 f2480f2ac0           	cvtsi2sd	xmm0, rax
;;   3d:	 f20f58c0             	addsd	xmm0, xmm0
;;   41:	 4883ec08             	sub	rsp, 8
;;   45:	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;   4a:	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;   4f:	 4883c408             	add	rsp, 8
;;   53:	 4883c408             	add	rsp, 8
;;   57:	 5d                   	pop	rbp
;;   58:	 c3                   	ret	
