;;! target = "x86_64"
(module
  (func (export "break-inner") (result i32)
    (local i32)
    (local.set 0 (i32.const 0))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (block (result i32) (br 2 (i32.const 0x1)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (loop (result i32) (br 2 (i32.const 0x2)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (block (result i32) (loop (result i32) (br 1 (i32.const 0x4))))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (i32.ctz (br 1 (i32.const 0x8)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (i32.ctz (loop (result i32) (br 2 (i32.const 0x10))))))))
    (local.get 0)
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c31c000000       	addq	$0x1c, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f87c7000000         	ja	0xe2
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 b800000000           	movl	$0, %eax
;;      	 89442404             	movl	%eax, 4(%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 b801000000           	movl	$1, %eax
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 01c1                 	addl	%eax, %ecx
;;      	 894c2404             	movl	%ecx, 4(%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 b802000000           	movl	$2, %eax
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 01c1                 	addl	%eax, %ecx
;;      	 894c2404             	movl	%ecx, 4(%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 b804000000           	movl	$4, %eax
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 01c1                 	addl	%eax, %ecx
;;      	 894c2404             	movl	%ecx, 4(%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 b808000000           	movl	$8, %eax
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 01c1                 	addl	%eax, %ecx
;;      	 894c2404             	movl	%ecx, 4(%rsp)
;;      	 448b5c2404           	movl	4(%rsp), %r11d
;;      	 4883ec04             	subq	$4, %rsp
;;      	 44891c24             	movl	%r11d, (%rsp)
;;      	 b810000000           	movl	$0x10, %eax
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 01c1                 	addl	%eax, %ecx
;;      	 894c2404             	movl	%ecx, 4(%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   e2:	 0f0b                 	ud2	
