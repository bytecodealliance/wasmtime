;;! target = "x86_64"

(module
    (func (result f64)
        (local i32)  

        (local.get 0)
        (f64.convert_i32_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   19:	 8bc9                 	mov	ecx, ecx
;;   1b:	 4883f900             	cmp	rcx, 0
;;   1f:	 0f8c0a000000         	jl	0x2f
;;   25:	 f2480f2ac1           	cvtsi2sd	xmm0, rcx
;;   2a:	 e91a000000           	jmp	0x49
;;   2f:	 4989cb               	mov	r11, rcx
;;   32:	 49c1eb01             	shr	r11, 1
;;   36:	 4889c8               	mov	rax, rcx
;;   39:	 4883e001             	and	rax, 1
;;   3d:	 4c09d8               	or	rax, r11
;;   40:	 f2480f2ac0           	cvtsi2sd	xmm0, rax
;;   45:	 f20f58c0             	addsd	xmm0, xmm0
;;   49:	 4883c410             	add	rsp, 0x10
;;   4d:	 5d                   	pop	rbp
;;   4e:	 c3                   	ret	
