;;! target = "x86_64"

(module
    (func (result f32)
        (local i64)  

        (local.get 0)
        (f32.convert_i64_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;   1a:	 4883f900             	cmp	rcx, 0
;;   1e:	 0f8c0a000000         	jl	0x2e
;;   24:	 f3480f2ac1           	cvtsi2ss	xmm0, rcx
;;   29:	 e91a000000           	jmp	0x48
;;   2e:	 4989cb               	mov	r11, rcx
;;   31:	 49c1eb01             	shr	r11, 1
;;   35:	 4889c8               	mov	rax, rcx
;;   38:	 4883e001             	and	rax, 1
;;   3c:	 4c09d8               	or	rax, r11
;;   3f:	 f3480f2ac0           	cvtsi2ss	xmm0, rax
;;   44:	 f30f58c0             	addss	xmm0, xmm0
;;   48:	 4883c410             	add	rsp, 0x10
;;   4c:	 5d                   	pop	rbp
;;   4d:	 c3                   	ret	
