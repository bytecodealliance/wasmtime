;;! target = "x86_64"
(module
  (func (;0;) (param i32)
    local.get 0
    block ;; label = @1
      i32.const 808727609
      br_table 0 (;@1;) 1 (;@0;) 0 (;@1;)
    end
    drop
  )
  (export "main" (func 0))
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c31c000000       	addq	$0x1c, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8766000000         	ja	0x81
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 b839343430           	movl	$0x30343439, %eax
;;      	 b902000000           	movl	$2, %ecx
;;      	 39c1                 	cmpl	%eax, %ecx
;;      	 0f42c1               	cmovbl	%ecx, %eax
;;      	 4c8d1d0a000000       	leaq	0xa(%rip), %r11
;;      	 49630c83             	movslq	(%r11, %rax, 4), %rcx
;;      	 4901cb               	addq	%rcx, %r11
;;      	 41ffe3               	jmpq	*%r11
;;   5d:	 1a00                 	sbbb	(%rax), %al
;;      	 0000                 	addb	%al, (%rax)
;;      	 1100                 	adcl	%eax, (%rax)
;;      	 0000                 	addb	%al, (%rax)
;;      	 1a00                 	sbbb	(%rax), %al
;;      	 0000                 	addb	%al, (%rax)
;;      	 e909000000           	jmp	0x77
;;   6e:	 4883c404             	addq	$4, %rsp
;;      	 e904000000           	jmp	0x7b
;;   77:	 4883c404             	addq	$4, %rsp
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   81:	 0f0b                 	ud2	
