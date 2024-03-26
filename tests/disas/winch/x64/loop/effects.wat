;;! target = "x86_64"
;;! test = "winch"
(module
  (func $fx (export "effects") (result i32)
    (local i32)
    (block
      (loop
        (local.set 0 (i32.const 1))
        (local.set 0 (i32.mul (local.get 0) (i32.const 3)))
        (local.set 0 (i32.sub (local.get 0) (i32.const 5)))
        (local.set 0 (i32.mul (local.get 0) (i32.const 7)))
        (br 1)
        (local.set 0 (i32.mul (local.get 0) (i32.const 100)))
      )
    )
    (i32.eq (local.get 0) (i32.const -14))
  )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f8759000000         	ja	0x74
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 b801000000           	movl	$1, %eax
;;      	 89442404             	movl	%eax, 4(%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 6bc003               	imull	$3, %eax, %eax
;;      	 89442404             	movl	%eax, 4(%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 83e805               	subl	$5, %eax
;;      	 89442404             	movl	%eax, 4(%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 6bc007               	imull	$7, %eax, %eax
;;      	 89442404             	movl	%eax, 4(%rsp)
;;      	 8b442404             	movl	4(%rsp), %eax
;;      	 83f8f2               	cmpl	$-0xe, %eax
;;      	 b800000000           	movl	$0, %eax
;;      	 400f94c0             	sete	%al
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   74:	 0f0b                 	ud2	
