;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.abs)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8732000000         	ja	0x4d
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f10051d000000     	movsd	0x1d(%rip), %xmm0
;;      	 49bbffffffffffffff7f 	
;; 				movabsq	$0x7fffffffffffffff, %r11
;;      	 664d0f6efb           	movq	%r11, %xmm15
;;      	 66410f54c7           	andpd	%xmm15, %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   4d:	 0f0b                 	ud2	
;;   4f:	 001f                 	addb	%bl, (%rdi)
;;   51:	 85eb                 	testl	%ebp, %ebx
;;   53:	 51                   	pushq	%rcx
