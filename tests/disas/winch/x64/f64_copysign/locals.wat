;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (local $foo f64)  
        (local $bar f64)

        (f64.const -1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.copysign
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f876f000000         	ja	0x8a
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec20             	subq	$0x20, %rsp
;;      	 48897c2418           	movq	%rdi, 0x18(%rsp)
;;      	 4889742410           	movq	%rsi, 0x10(%rsp)
;;      	 4531db               	xorl	%r11d, %r11d
;;      	 4c895c2408           	movq	%r11, 8(%rsp)
;;      	 4c891c24             	movq	%r11, (%rsp)
;;      	 f20f100550000000     	movsd	0x50(%rip), %xmm0
;;      	 f20f11442408         	movsd	%xmm0, 8(%rsp)
;;      	 f20f10054a000000     	movsd	0x4a(%rip), %xmm0
;;      	 f20f110424           	movsd	%xmm0, (%rsp)
;;      	 f20f100424           	movsd	(%rsp), %xmm0
;;      	 f20f104c2408         	movsd	8(%rsp), %xmm1
;;      	 49bb0000000000000080 	
;; 				movabsq	$9223372036854775808, %r11
;;      	 664d0f6efb           	movq	%r11, %xmm15
;;      	 66410f54c7           	andpd	%xmm15, %xmm0
;;      	 66440f55f9           	andnpd	%xmm1, %xmm15
;;      	 66410f28cf           	movapd	%xmm15, %xmm1
;;      	 660f56c8             	orpd	%xmm0, %xmm1
;;      	 660f28c1             	movapd	%xmm1, %xmm0
;;      	 4883c420             	addq	$0x20, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   8a:	 0f0b                 	ud2	
;;   8c:	 0000                 	addb	%al, (%rax)
;;   8e:	 0000                 	addb	%al, (%rax)
