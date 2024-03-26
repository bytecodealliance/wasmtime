;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local f64)  

        (local.get 0)
        (i64.trunc_f64_u)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8779000000         	ja	0x94
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 f20f100c24           	movsd	(%rsp), %xmm1
;;      	 49bb000000000000e043 	
;; 				movabsq	$0x43e0000000000000, %r11
;;      	 664d0f6efb           	movq	%r11, %xmm15
;;      	 66410f2ecf           	ucomisd	%xmm15, %xmm1
;;      	 0f8317000000         	jae	0x6a
;;      	 0f8a3d000000         	jp	0x96
;;   59:	 f2480f2cc1           	cvttsd2si	%xmm1, %rax
;;      	 4883f800             	cmpq	$0, %rax
;;      	 0f8d26000000         	jge	0x8e
;;   68:	 0f0b                 	ud2	
;;      	 0f28c1               	movaps	%xmm1, %xmm0
;;      	 f2410f5cc7           	subsd	%xmm15, %xmm0
;;      	 f2480f2cc0           	cvttsd2si	%xmm0, %rax
;;      	 4883f800             	cmpq	$0, %rax
;;      	 0f8c17000000         	jl	0x98
;;   81:	 49bb0000000000000080 	
;; 				movabsq	$9223372036854775808, %r11
;;      	 4c01d8               	addq	%r11, %rax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   94:	 0f0b                 	ud2	
;;   96:	 0f0b                 	ud2	
;;   98:	 0f0b                 	ud2	
