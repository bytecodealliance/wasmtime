;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        f64.const 1.0
        i64.reinterpret_f64
        drop
        f64.const 1.0
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f872b000000         	ja	0x46
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f100515000000     	movsd	0x15(%rip), %xmm0
;;      	 66480f7ec0           	movq	%xmm0, %rax
;;      	 f20f100508000000     	movsd	8(%rip), %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   46:	 0f0b                 	ud2	
;;   48:	 0000                 	addb	%al, (%rax)
;;   4a:	 0000                 	addb	%al, (%rax)
;;   4c:	 0000                 	addb	%al, (%rax)
