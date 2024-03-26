;;! target = "x86_64"

(module
  (memory (data "\00\00\00\00\00\00\f4\7f"))
  (func (export "f64.store") (f64.store (i32.const 0) (f64.const nan:0x4000000000000)))
)

;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f872e000000         	ja	0x49
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 f20f10051d000000     	movsd	0x1d(%rip), %xmm0
;;      	 b800000000           	movl	$0, %eax
;;      	 498b4e50             	movq	0x50(%r14), %rcx
;;      	 4801c1               	addq	%rax, %rcx
;;      	 f20f1101             	movsd	%xmm0, (%rcx)
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   49:	 0f0b                 	ud2	
;;   4b:	 0000                 	addb	%al, (%rax)
;;   4d:	 0000                 	addb	%al, (%rax)
;;   4f:	 0000                 	addb	%al, (%rax)
;;   51:	 0000                 	addb	%al, (%rax)
;;   53:	 0000                 	addb	%al, (%rax)
;;   55:	 00f4                 	addb	%dh, %ah
