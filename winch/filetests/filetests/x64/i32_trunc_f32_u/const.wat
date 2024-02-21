;;! target = "x86_64"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.trunc_f32_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8760000000         	ja	0x7e
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 f30f100d55000000     	movss	xmm1, dword ptr [rip + 0x55]
;;      	 41bb0000004f         	mov	r11d, 0x4f000000
;;      	 66450f6efb           	movd	xmm15, r11d
;;      	 410f2ecf             	ucomiss	xmm1, xmm15
;;      	 0f8315000000         	jae	0x5d
;;      	 0f8a32000000         	jp	0x80
;;   4e:	 f30f2cc1             	cvttss2si	eax, xmm1
;;      	 83f800               	cmp	eax, 0
;;      	 0f8d1d000000         	jge	0x78
;;   5b:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	xmm0, xmm1
;;      	 f3410f5cc7           	subss	xmm0, xmm15
;;      	 f30f2cc0             	cvttss2si	eax, xmm0
;;      	 83f800               	cmp	eax, 0
;;      	 0f8c10000000         	jl	0x82
;;   72:	 81c000000080         	add	eax, 0x80000000
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   7e:	 0f0b                 	ud2	
;;   80:	 0f0b                 	ud2	
;;   82:	 0f0b                 	ud2	
;;   84:	 0000                 	add	byte ptr [rax], al
;;   86:	 0000                 	add	byte ptr [rax], al
;;   88:	 0000                 	add	byte ptr [rax], al
