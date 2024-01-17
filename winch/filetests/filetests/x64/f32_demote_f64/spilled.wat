;;! target = "x86_64"

(module
    (func (result f32)
        f64.const 1.0
        f32.demote_f64
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
;;      	 f20f100524000000     	movsd	xmm0, qword ptr [rip + 0x24]
;;      	 f20f5ac0             	cvtsd2ss	xmm0, xmm0
;;      	 4883ec04             	sub	rsp, 4
;;      	 f30f110424           	movss	dword ptr [rsp], xmm0
;;      	 f30f100424           	movss	xmm0, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   40:	 0f0b                 	ud2	
;;   42:	 0000                 	add	byte ptr [rax], al
;;   44:	 0000                 	add	byte ptr [rax], al
;;   46:	 0000                 	add	byte ptr [rax], al
;;   48:	 0000                 	add	byte ptr [rax], al
;;   4a:	 0000                 	add	byte ptr [rax], al
;;   4c:	 0000                 	add	byte ptr [rax], al
