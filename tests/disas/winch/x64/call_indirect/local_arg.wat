;;! target="x86_64"

(module
    (type $param-i32 (func (param i32)))

    (func $param-i32 (type $param-i32))
    (func (export "")
        (local i32)
        local.get 0
        (call_indirect (type $param-i32) (i32.const 0))
    )

    (table funcref
      (elem
        $param-i32)
    )
)

;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f871b000000         	ja	0x36
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f87cb000000         	ja	0xe6
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 b900000000           	movl	$0, %ecx
;;      	 4c89f2               	movq	%r14, %rdx
;;      	 8b5a50               	movl	0x50(%rdx), %ebx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 0f8394000000         	jae	0xe8
;;   54:	 4189cb               	movl	%ecx, %r11d
;;      	 4d6bdb08             	imulq	$8, %r11, %r11
;;      	 488b5248             	movq	0x48(%rdx), %rdx
;;      	 4889d6               	movq	%rdx, %rsi
;;      	 4c01da               	addq	%r11, %rdx
;;      	 39d9                 	cmpl	%ebx, %ecx
;;      	 480f43d6             	cmovaeq	%rsi, %rdx
;;      	 488b02               	movq	(%rdx), %rax
;;      	 4885c0               	testq	%rax, %rax
;;      	 0f8525000000         	jne	0x9c
;;   77:	 4883ec04             	subq	$4, %rsp
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 8b1424               	movl	(%rsp), %edx
;;      	 e800000000           	callq	0x8e
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742414           	movq	0x14(%rsp), %r14
;;      	 e904000000           	jmp	0xa0
;;   9c:	 4883e0fe             	andq	$0xfffffffffffffffe, %rax
;;      	 4885c0               	testq	%rax, %rax
;;      	 0f8441000000         	je	0xea
;;   a9:	 4d8b5e40             	movq	0x40(%r14), %r11
;;      	 418b0b               	movl	(%r11), %ecx
;;      	 8b5018               	movl	0x18(%rax), %edx
;;      	 39d1                 	cmpl	%edx, %ecx
;;      	 0f8531000000         	jne	0xec
;;   bb:	 488b5820             	movq	0x20(%rax), %rbx
;;      	 488b4810             	movq	0x10(%rax), %rcx
;;      	 4883ec04             	subq	$4, %rsp
;;      	 4889df               	movq	%rbx, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 8b542404             	movl	4(%rsp), %edx
;;      	 ffd1                 	callq	*%rcx
;;      	 4883c404             	addq	$4, %rsp
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   e6:	 0f0b                 	ud2	
;;   e8:	 0f0b                 	ud2	
;;   ea:	 0f0b                 	ud2	
;;   ec:	 0f0b                 	ud2	
