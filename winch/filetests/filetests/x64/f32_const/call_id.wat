;;! target = "x86_64"

(module
  (func $id-f32 (param f32) (result f32) (local.get 0))
  (func (export "type-first-f32") (result f32) (call $id-f32 (f32.const 1.32)))
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8723000000         	ja	0x3e
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 f30f11442404         	movss	%xmm0, 4(%rsp)
;;      	 f30f10442404         	movss	4(%rsp), %xmm0
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   3e:	 0f0b                 	ud2	
;;
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
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 f30f100517000000     	movss	0x17(%rip), %xmm0
;;      	 e800000000           	callq	0x3e
;;      	 4c8b742408           	movq	8(%rsp), %r14
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   49:	 0f0b                 	ud2	
;;   4b:	 0000                 	addb	%al, (%rax)
;;   4d:	 0000                 	addb	%al, (%rax)
;;   4f:	 00c3                 	addb	%al, %bl
;;   51:	 f5                   	cmc	
;;   52:	 a83f                 	testb	$0x3f, %al
