;;! target = "x86_64"

(module
    (func (result f64)
        i32.const 1
        f64.convert_i32_u
        block
        end
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8751000000         	ja	0x69
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b901000000           	mov	ecx, 1
;;      	 8bc9                 	mov	ecx, ecx
;;      	 4883f900             	cmp	rcx, 0
;;      	 0f8c0a000000         	jl	0x37
;;   2d:	 f2480f2ac1           	cvtsi2sd	xmm0, rcx
;;      	 e91a000000           	jmp	0x51
;;   37:	 4989cb               	mov	r11, rcx
;;      	 49c1eb01             	shr	r11, 1
;;      	 4889c8               	mov	rax, rcx
;;      	 4883e001             	and	rax, 1
;;      	 4c09d8               	or	rax, r11
;;      	 f2480f2ac0           	cvtsi2sd	xmm0, rax
;;      	 f20f58c0             	addsd	xmm0, xmm0
;;      	 4883ec08             	sub	rsp, 8
;;      	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;      	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;      	 4883c408             	add	rsp, 8
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   69:	 0f0b                 	ud2	
