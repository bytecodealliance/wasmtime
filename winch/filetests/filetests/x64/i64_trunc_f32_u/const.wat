;;! target = "x86_64"

(module
    (func (result i64)
        (f32.const 1.0)
        (i64.trunc_f32_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f876b000000         	ja	0x89
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 f30f100d5d000000     	movss	xmm1, dword ptr [rip + 0x5d]
;;      	 41bb0000005f         	mov	r11d, 0x5f000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f2ecf             	ucomiss	xmm1, xmm15
;;      	 0f8317000000         	jae	0x5f
;;      	 0f8a3d000000         	jp	0x8b
;;   4e:	 f3480f2cc1           	cvttss2si	rax, xmm1
;;      	 4883f800             	cmp	rax, 0
;;      	 0f8d26000000         	jge	0x83
;;   5d:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 f3410f5cc7           	subss	xmm0, xmm15
;;      	 f3480f2cc0           	cvttss2si	rax, xmm0
;;      	 4883f800             	cmp	rax, 0
;;      	 0f8c17000000         	jl	0x8d
;;   76:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;      	 4c01d8               	add	rax, r11
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   89:	 0f0b                 	ud2	
;;   8b:	 0f0b                 	ud2	
;;   8d:	 0f0b                 	ud2	
;;   8f:	 0000                 	add	byte ptr [rax], al
