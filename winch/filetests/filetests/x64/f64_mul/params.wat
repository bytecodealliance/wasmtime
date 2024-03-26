;;! target = "x86_64"

(module
    (func (param f64) (param f64) (result f64)
        (local.get 0)
        (local.get 1)
        (f64.mul)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8735000000         	ja	0x50
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 f20f11442408         	movsd	%xmm0, 8(%rsp)
;;      	 f20f110c24           	movsd	%xmm1, (%rsp)
;;      	 f20f100424           	movsd	(%rsp), %xmm0
;;      	 f20f104c2408         	movsd	8(%rsp), %xmm1
;;      	 f20f59c8             	mulsd	%xmm0, %xmm1
;;      	 660f28c1             	movapd	%xmm1, %xmm0
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   50:	 0f0b                 	ud2	
