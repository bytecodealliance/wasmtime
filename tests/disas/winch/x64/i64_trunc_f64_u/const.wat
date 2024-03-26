;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.trunc_f64_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8773000000         	ja	0x8e
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f100d65000000     	movsd	0x65(%rip), %xmm1
;;      	 49bb000000000000e043 	
;; 				movabsq	$0x43e0000000000000, %r11
;;      	 664d0f6efb           	movq	%r11, %xmm15
;;      	 66410f2ecf           	ucomisd	%xmm15, %xmm1
;;      	 0f8317000000         	jae	0x64
;;      	 0f8a3d000000         	jp	0x90
;;   53:	 f2480f2cc1           	cvttsd2si	%xmm1, %rax
;;      	 4883f800             	cmpq	$0, %rax
;;      	 0f8d26000000         	jge	0x88
;;   62:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 f2410f5cc7           	subsd	%xmm15, %xmm0
;;      	 f2480f2cc0           	cvttsd2si	%xmm0, %rax
;;      	 4883f800             	cmpq	$0, %rax
;;      	 0f8c17000000         	jl	0x92
;;   7b:	 49bb0000000000000080 	
;; 				movabsq	$9223372036854775808, %r11
;;      	 4c01d8               	addq	%r11, %rax
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   8e:	 0f0b                 	ud2	
;;   90:	 0f0b                 	ud2	
;;   92:	 0f0b                 	ud2	
;;   94:	 0000                 	addb	%al, (%rax)
;;   96:	 0000                 	addb	%al, (%rax)
;;   98:	 0000                 	addb	%al, (%rax)
;;   9a:	 0000                 	addb	%al, (%rax)
;;   9c:	 0000                 	addb	%al, (%rax)
