;;! target = "x86_64"
;;! test = "winch"

(module
  (memory (data "\00\00\a0\7f"))

  (func (export "f32.load") (result f32) (f32.load (i32.const 0)))
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8726000000         	ja	0x41
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 b800000000           	movl	$0, %eax
;;      	 498b4e50             	movq	0x50(%r14), %rcx
;;      	 4801c1               	addq	%rax, %rcx
;;      	 f30f1001             	movss	(%rcx), %xmm0
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   41:	 0f0b                 	ud2	
