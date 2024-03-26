;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const 1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.ne
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f875e000000         	ja	0x79
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 f30f100544000000     	movss	0x44(%rip), %xmm0
;;      	 f30f11442404         	movss	%xmm0, 4(%rsp)
;;      	 f30f10053e000000     	movss	0x3e(%rip), %xmm0
;;      	 f30f110424           	movss	%xmm0, (%rsp)
;;      	 f30f100424           	movss	(%rsp), %xmm0
;;      	 f30f104c2404         	movss	4(%rsp), %xmm1
;;      	 0f2ec8               	ucomiss	%xmm0, %xmm1
;;      	 b800000000           	movl	$0, %eax
;;      	 400f95c0             	setne	%al
;;      	 41bb00000000         	movl	$0, %r11d
;;      	 410f9ac3             	setp	%r11b
;;      	 4409d8               	orl	%r11d, %eax
;;      	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   79:	 0f0b                 	ud2	
;;   7b:	 0000                 	addb	%al, (%rax)
;;   7d:	 0000                 	addb	%al, (%rax)
;;   7f:	 00cd                 	addb	%cl, %ch
;;   81:	 cc                   	int3	
