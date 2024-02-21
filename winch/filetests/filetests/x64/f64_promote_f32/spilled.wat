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
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8731000000         	ja	0x4f
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 f30f100525000000     	movss	xmm0, dword ptr [rip + 0x25]
;;      	 f30f5ac0             	cvtss2sd	xmm0, xmm0
;;      	 4883ec08             	sub	rsp, 8
;;      	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;      	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;      	 4883c408             	add	rsp, 8
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   4f:	 0f0b                 	ud2	
;;   51:	 0000                 	add	byte ptr [rax], al
;;   53:	 0000                 	add	byte ptr [rax], al
;;   55:	 0000                 	add	byte ptr [rax], al
;;   57:	 0000                 	add	byte ptr [rax], al
