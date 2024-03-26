;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local f32)  

        (local.get 0)
        (i32.trunc_f32_s)
    )
)
;;      	 55                   	pushq	%rbp
;;      	 4889e5               	movq	%rsp, %rbp
;;      	 4c8b5f08             	movq	8(%rdi), %r11
;;      	 4d8b1b               	movq	(%r11), %r11
;;      	 4981c318000000       	addq	$0x18, %r11
;;      	 4939e3               	cmpq	%rsp, %r11
;;      	 0f875f000000         	ja	0x7a
;;   1b:	 4989fe               	movq	%rdi, %r14
;;      	 4883ec18             	subq	$0x18, %rsp
;;      	 48897c2410           	movq	%rdi, 0x10(%rsp)
;;      	 4889742408           	movq	%rsi, 8(%rsp)
;;      	 48c7042400000000     	movq	$0, (%rsp)
;;      	 f30f10442404         	movss	4(%rsp), %xmm0
;;      	 f30f2cc0             	cvttss2si	%xmm0, %eax
;;      	 83f801               	cmpl	$1, %eax
;;      	 0f812d000000         	jno	0x74
;;   47:	 0f2ec0               	ucomiss	%xmm0, %xmm0
;;      	 0f8a2c000000         	jp	0x7c
;;   50:	 41bb000000cf         	movl	$0xcf000000, %r11d
;;      	 66450f6efb           	movd	%r11d, %xmm15
;;      	 410f2ec7             	ucomiss	%xmm15, %xmm0
;;      	 0f8219000000         	jb	0x7e
;;   65:	 66450f57ff           	xorpd	%xmm15, %xmm15
;;      	 440f2ef8             	ucomiss	%xmm0, %xmm15
;;      	 0f820c000000         	jb	0x80
;;   74:	 4883c418             	addq	$0x18, %rsp
;;      	 5d                   	popq	%rbp
;;      	 c3                   	retq	
;;   7a:	 0f0b                 	ud2	
;;   7c:	 0f0b                 	ud2	
;;   7e:	 0f0b                 	ud2	
;;   80:	 0f0b                 	ud2	
