;;! target = "x86_64"

(module
    (func (result f32)
        i32.const 1
        f32.convert_i32_u
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
;;   2d:	 f3480f2ac1           	cvtsi2ss	xmm0, rcx
;;      	 e91a000000           	jmp	0x51
;;   37:	 4989cb               	mov	r11, rcx
;;      	 49c1eb01             	shr	r11, 1
;;      	 4889c8               	mov	rax, rcx
;;      	 4883e001             	and	rax, 1
;;      	 4c09d8               	or	rax, r11
;;      	 f3480f2ac0           	cvtsi2ss	xmm0, rax
;;      	 f30f58c0             	addss	xmm0, xmm0
;;      	 4883ec04             	sub	rsp, 4
;;      	 f30f110424           	movss	dword ptr [rsp], xmm0
;;      	 f30f100424           	movss	xmm0, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   69:	 0f0b                 	ud2	
