;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.floor)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8747000000         	ja	0x62
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 f20f110424           	movsd	%xmm0, (%rsp)
;;      	 f2440f103c24         	movsd	(%rsp), %xmm15
;;      	 4883ec08             	subq	$8, %rsp
;;      	 f2440f113c24         	movsd	%xmm15, (%rsp)
;;      	 f20f100424           	movsd	(%rsp), %xmm0
;;      	 49bb0000000000000000 	
;; 				movabsq	$0, %r11
;;      	 41ffd3               	callq	*%r11
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   62:	 0f0b                 	ud2	
