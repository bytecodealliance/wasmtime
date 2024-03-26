;;! target = "x86_64"
;;! test = "winch"
(module
  (func $dummy)
  (func (export "nested") (param i32 i32) (result i32)
    (if (result i32) (local.get 0)
      (then
        (if (local.get 1) (then (call $dummy) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 9))
          (else (call $dummy) (i32.const 10))
        )
      )
      (else
        (if (local.get 1) (then (call $dummy) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 10))
          (else (call $dummy) (i32.const 11))
        )
      )
    )
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c310000000       	addq	$0x10, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8716000000         	ja	0x31
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec10             	subq	$0x10, %rsp
;;      	 48897c2408           	movq	%rdi, 8(%rsp)
;;      	 48893424             	movq	%rsi, (%rsp)
;;      	 4883c410             	addq	$0x10, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   31:	 0f0b                 	ud2	
;;
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8759010000         	ja	0x174
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f849a000000         	je	0xd9
;;   3f:	 8b0424               	movl	(%rsp), %eax
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f8418000000         	je	0x62
;;   4a:	 4883ec08             	subq	$8, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 e800000000           	callq	0x59
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f8405000000         	je	0x72
;;      	 e918000000           	jmp	0x8a
;;   72:	 4883ec08             	subq	$8, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 e800000000           	callq	0x81
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f8422000000         	je	0xb7
;;   95:	 4883ec08             	subq	$8, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 e800000000           	callq	0xa4
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 b809000000           	movl	$9, %eax
;;      	 e9b7000000           	jmp	0x16e
;;   b7:	 4883ec08             	subq	$8, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 e800000000           	callq	0xc6
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 b80a000000           	movl	$0xa, %eax
;;      	 e995000000           	jmp	0x16e
;;   d9:	 8b0424               	movl	(%rsp), %eax
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f8418000000         	je	0xfc
;;   e4:	 4883ec08             	subq	$8, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 e800000000           	callq	0xf3
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f8405000000         	je	0x10c
;;      	 e918000000           	jmp	0x124
;;  10c:	 4883ec08             	subq	$8, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 e800000000           	callq	0x11b
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 8b0424               	movl	(%rsp), %eax
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f8422000000         	je	0x151
;;  12f:	 4883ec08             	subq	$8, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 e800000000           	callq	0x13e
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 b80a000000           	movl	$0xa, %eax
;;      	 e91d000000           	jmp	0x16e
;;  151:	 4883ec08             	subq	$8, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 e800000000           	callq	0x160
;;      	 4883c408             	addq	$8, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 b80b000000           	movl	$0xb, %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;  174:	 0f0b                 	ud2	
