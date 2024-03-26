;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (result f32)
        (local.get 0)
        (f32.trunc)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8752000000         	ja	0x6d
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 f30f11442404         	movss	%xmm0, 4(%rsp)
;;      	 f3440f107c2404       	movss	4(%rsp), %xmm15
;;      	 4883ec04             	subq	$4, %rsp
;;      	 f3440f113c24         	movss	%xmm15, (%rsp)
;;      	 4883ec04             	subq	$4, %rsp
;;      	 f30f10442404         	movss	4(%rsp), %xmm0
;;      	 49bb0000000000000000 	
;; 				movabsq	$0, %r11
;;      	 41ffd3               	callq	*%r11
;;      	 4883c404             	addq	$4, %rsp
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   6d:	 0f0b                 	ud2	
