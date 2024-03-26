;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local f32)  

        (local.get 0)
        (i64.trunc_f32_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8775000000         	ja	0x90
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 f30f104c2404         	movss	4(%rsp), %xmm1
;;      	 41bb0000005f         	movl	$0x5f000000, %r11d
;;      	 66450f6efb           	movd	%r11d, %xmm15
;;      	 410f2ecf             	ucomiss	%xmm15, %xmm1
;;      	 0f8317000000         	jae	0x66
;;      	 0f8a3d000000         	jp	0x92
;;   55:	 f3480f2cc1           	cvttss2si	%xmm1, %rax
;;      	 4883f800             	cmpq	$0, %rax
;;      	 0f8d26000000         	jge	0x8a
;;   64:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 f3410f5cc7           	subss	%xmm15, %xmm0
;;      	 f3480f2cc0           	cvttss2si	%xmm0, %rax
;;      	 4883f800             	cmpq	$0, %rax
;;      	 0f8c17000000         	jl	0x94
;;   7d:	 49bb0000000000000080 	
;; 				movabsq	$9223372036854775808, %r11
;;      	 4c01d8               	addq	%r11, %rax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   90:	 0f0b                 	ud2	
;;   92:	 0f0b                 	ud2	
;;   94:	 0f0b                 	ud2	
