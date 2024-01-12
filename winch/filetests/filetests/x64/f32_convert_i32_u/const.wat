;;! target = "x86_64"

(module
    (func (result f32)
        (i32.const 1)
        (f32.convert_i32_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b901000000           	mov	ecx, 1
;;   11:	 8bc9                 	mov	ecx, ecx
;;   13:	 4883f900             	cmp	rcx, 0
;;   17:	 0f8c0a000000         	jl	0x27
;;   1d:	 f3480f2ac1           	cvtsi2ss	xmm0, rcx
;;   22:	 e91a000000           	jmp	0x41
;;   27:	 4989cb               	mov	r11, rcx
;;   2a:	 49c1eb01             	shr	r11, 1
;;   2e:	 4889c8               	mov	rax, rcx
;;   31:	 4883e001             	and	rax, 1
;;   35:	 4c09d8               	or	rax, r11
;;   38:	 f3480f2ac0           	cvtsi2ss	xmm0, rax
;;   3d:	 f30f58c0             	addss	xmm0, xmm0
;;   41:	 4883c408             	add	rsp, 8
;;   45:	 5d                   	pop	rbp
;;   46:	 c3                   	ret	
