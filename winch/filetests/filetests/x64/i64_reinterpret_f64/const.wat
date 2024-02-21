;;! target = "x86_64"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.reinterpret_f64)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8720000000         	ja	0x3e
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 f20f10050d000000     	movsd	xmm0, qword ptr [rip + 0xd]
;;      	 66480f7ec0           	movq	rax, xmm0
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3e:	 0f0b                 	ud2	
;;   40:	 0000                 	add	byte ptr [rax], al
;;   42:	 0000                 	add	byte ptr [rax], al
;;   44:	 0000                 	add	byte ptr [rax], al
