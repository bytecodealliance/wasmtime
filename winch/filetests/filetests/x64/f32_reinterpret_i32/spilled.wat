;;! target = "x86_64"

(module
    (func (result f32)
        i32.const 1
        f32.reinterpret_i32
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
;;      	 0f8725000000         	ja	0x3d
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b801000000           	mov	eax, 1
;;      	 660f6ec0             	movd	xmm0, eax
;;      	 4883ec04             	sub	rsp, 4
;;      	 f30f110424           	movss	dword ptr [rsp], xmm0
;;      	 f30f100424           	movss	xmm0, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3d:	 0f0b                 	ud2	
