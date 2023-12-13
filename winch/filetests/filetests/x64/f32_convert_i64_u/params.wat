;;! target = "x86_64"

(module
    (func (param i64) (result f32)
        (local.get 0)
        (f32.convert_i64_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;    d:	 4c893424             	mov	qword ptr [rsp], r14
;;   11:	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;   16:	 4883f900             	cmp	rcx, 0
;;   1a:	 0f8c0a000000         	jl	0x2a
;;   20:	 f3480f2ac1           	cvtsi2ss	xmm0, rcx
;;   25:	 e91a000000           	jmp	0x44
;;   2a:	 4989cb               	mov	r11, rcx
;;   2d:	 49c1eb01             	shr	r11, 1
;;   31:	 4889c8               	mov	rax, rcx
;;   34:	 4883e001             	and	rax, 1
;;   38:	 4c09d8               	or	rax, r11
;;   3b:	 f3480f2ac0           	cvtsi2ss	xmm0, rax
;;   40:	 f30f58c0             	addss	xmm0, xmm0
;;   44:	 4883c410             	add	rsp, 0x10
;;   48:	 5d                   	pop	rbp
;;   49:	 c3                   	ret	
