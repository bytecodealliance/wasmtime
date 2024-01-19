;;! target = "x86_64"

(module
    (func (result f64)
        f32.const 1.0
        f64.promote_f32
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
;;      	 0f8728000000         	ja	0x40
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f100524000000     	movss	xmm0, dword ptr [rip + 0x24]
;;      	 f30f5ac0             	cvtss2sd	xmm0, xmm0
;;      	 4883ec08             	sub	rsp, 8
;;      	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;      	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;      	 4883c408             	add	rsp, 8
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   40:	 0f0b                 	ud2	
;;   42:	 0000                 	add	byte ptr [rax], al
;;   44:	 0000                 	add	byte ptr [rax], al
;;   46:	 0000                 	add	byte ptr [rax], al
;;   48:	 0000                 	add	byte ptr [rax], al
