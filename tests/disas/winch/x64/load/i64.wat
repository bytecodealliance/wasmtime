;;! target = "x86_64"
(module
  (memory 1)
  (func (export "i64_load8_s") (param $i i64) (result i64)
   (i64.store8 (i32.const 8) (local.get $i))
   (i64.load8_s (i32.const 8))
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f873d000000         	ja	0x58
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48891424             	movq	%rdx, (%rsp)
;;      	 488b0424             	movq	(%rsp), %rax
;;      	 b908000000           	movl	$8, %ecx
;;      	 498b5650             	movq	0x50(%r14), %rdx
;;      	 4801ca               	addq	%rcx, %rdx
;;      	 8802                 	movb	%al, (%rdx)
;;      	 b808000000           	movl	$8, %eax
;;      	 498b4e50             	movq	0x50(%r14), %rcx
;;      	 4801c1               	addq	%rax, %rcx
;;      	 480fbe01             	movsbq	(%rcx), %rax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   58:	 0f0b                 	ud2	
