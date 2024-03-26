;;! target = "x86_64"

(module
    (func (result i64)
        (f32.const 1.0)
        (i64.trunc_f32_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f876e000000         	ja	0x89
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f30f100d5d000000     	movss	0x5d(%rip), %xmm1
;;      	 41bb0000005f         	movl	$0x5f000000, %r11d
;;      	 66450f6efb           	movd	%r11d, %xmm15
;;      	 410f2ecf             	ucomiss	%xmm15, %xmm1
;;      	 0f8317000000         	jae	0x5f
;;      	 0f8a3d000000         	jp	0x8b
;;   4e:	 f3480f2cc1           	cvttss2si	%xmm1, %rax
;;      	 4883f800             	cmpq	$0, %rax
;;      	 0f8d26000000         	jge	0x83
;;   5d:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 f3410f5cc7           	subss	%xmm15, %xmm0
;;      	 f3480f2cc0           	cvttss2si	%xmm0, %rax
;;      	 4883f800             	cmpq	$0, %rax
;;      	 0f8c17000000         	jl	0x8d
;;   76:	 49bb0000000000000080 	
;; 				movabsq	$9223372036854775808, %r11
;;      	 4c01d8               	addq	%r11, %rax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   89:	 0f0b                 	ud2	
;;   8b:	 0f0b                 	ud2	
;;   8d:	 0f0b                 	ud2	
;;   8f:	 0000                 	addb	%al, (%rax)
