;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const -1.1)
        (f64.const 2.2)
        (f64.copysign)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f874c000000         	ja	0x67
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f10053d000000     	movsd	0x3d(%rip), %xmm0
;;      	 f20f100d3d000000     	movsd	0x3d(%rip), %xmm1
;;      	 49bb0000000000000080 	
;; 				movabsq	$9223372036854775808, %r11
;;      	 664d0f6efb           	movq	%r11, %xmm15
;;      	 66410f54c7           	andpd	%xmm15, %xmm0
;;      	 66440f55f9           	andnpd	%xmm1, %xmm15
;;      	 66410f28cf           	movapd	%xmm15, %xmm1
;;      	 660f56c8             	orpd	%xmm0, %xmm1
;;      	 660f28c1             	movapd	%xmm1, %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   67:	 0f0b                 	ud2	
;;   69:	 0000                 	addb	%al, (%rax)
;;   6b:	 0000                 	addb	%al, (%rax)
;;   6d:	 0000                 	addb	%al, (%rax)
;;   6f:	 009a99999999         	addb	%bl, -0x66666667(%rdx)
;;   75:	 99                   	cltd	
;;   76:	 01409a               	addl	%eax, -0x66(%rax)
;;   79:	 99                   	cltd	
;;   7a:	 99                   	cltd	
;;   7b:	 99                   	cltd	
;;   7c:	 99                   	cltd	
;;   7d:	 99                   	cltd	
;;   7e:	 f1                   	int1	
