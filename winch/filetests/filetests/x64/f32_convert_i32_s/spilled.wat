;;! target = "x86_64"

(module
    (func (result f32)
        i32.const 1
        f32.convert_i32_s
        block
        end
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c314000000       	add	r11, 0x14
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872e000000         	ja	0x4c
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 b801000000           	mov	eax, 1
;;      	 f30f2ac0             	cvtsi2ss	xmm0, eax
;;      	 4883ec04             	sub	rsp, 4
;;      	 f30f110424           	movss	dword ptr [rsp], xmm0
;;      	 f30f100424           	movss	xmm0, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   4c:	 0f0b                 	ud2	
