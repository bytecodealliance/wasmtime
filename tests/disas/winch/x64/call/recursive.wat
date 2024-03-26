;;! target = "x86_64"
;;! test = "winch"

(module
  (func $fibonacci8 (param $n i32) (result i32)
    (if (result i32) (i32.le_s (local.get $n) (i32.const 1))
      (then
        ;; If n <= 1, return n (base case)
        (local.get $n)
      )
      (else
        ;; Else, return fibonacci(n - 1) + fibonacci(n - 2)
        (i32.add
          (call $fibonacci8
            (i32.sub (local.get $n) (i32.const 1)) ;; Calculate n - 1
          )
          (call $fibonacci8
            (i32.sub (local.get $n) (i32.const 2)) ;; Calculate n - 2
          )
        )
      )
    )
  )
  (export "fib" (func $fibonacci8))
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c320000000       	addq	$0x20, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f87a1000000         	ja	0xbc
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 89542404             	movl	%edx, 4(%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 83f801               	cmpl	$1, %eax
;;      	 b800000000           	movl	$0, %eax
;;      	 400f9ec0             	setle	%al
;;      	 85c0                 	testl	%eax, %eax
;;      	 0f8409000000         	je	0x51
;;   48:	 8b442404             	movl	4(%rsp), %eax
;;      	 e965000000           	jmp	0xb6
;;   51:	 8b442404             	movl	4(%rsp), %eax
;;      	 83e801               	subl	$1, %eax
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 4883ec04             	subq	$4, %rsp
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 8b542404             	movl	4(%rsp), %edx
;;      	 e800000000           	callq	0x72
;;      	 4883c404             	addq	$4, %rsp
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742410           	movq	0x10(%rsp), %r14
;;      	 8b4c2404             	movl	4(%rsp), %ecx
;;      	 83e902               	subl	$2, %ecx
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890424               	movl	%eax, (%rsp)
;;      	 4883ec04             	subq	$4, %rsp
;;      	 890c24               	movl	%ecx, (%rsp)
;;      	 4c89f7               	movq	%r14, %rdi
;;      	 4c89f6               	movq	%r14, %rsi
;;      	 8b1424               	movl	(%rsp), %edx
;;      	 e800000000           	callq	0xa2
;;      	 4883c404             	addq	$4, %rsp
;;      	 4c8b742414           	movq	0x14(%rsp), %r14
;;      	 8b0c24               	movl	(%rsp), %ecx
;;      	 4883c404             	addq	$4, %rsp
;;      	 01c1                 	addl	%eax, %ecx
;;      	 89c8                 	movl	%ecx, %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   bc:	 0f0b                 	ud2	
