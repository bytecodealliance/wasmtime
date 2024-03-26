;;! target = "x86_64"

(module
  (table $t1 0 funcref)

  (func (export "grow-by-10") (param $r funcref) (result i32)
    (table.grow $t1 (local.get $r) (i32.const 10))
  )
)


;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8740000000         	ja	0x5b
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48891424             	movq	%rdx, (%rsp)
;;      	 4c8b1c24             	movq	(%rsp), %r11
;;      	 4153                 	pushq	%r11
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 be00000000           	movl	$0, %esi
;;      	 ba0a000000           	movl	$0xa, %edx
;;      	 488b0c24             	movq	(%rsp), %rcx
;;      	 e800000000           	callq	0x4c
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   5b:	 0f0b                 	ud2	
