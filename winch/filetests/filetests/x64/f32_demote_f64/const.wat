;;! target = "x86_64"

(module
    (func (result f32)
        (f64.const 1.0)
        (f32.demote_f64)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f871f000000         	ja	0x3d
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 f20f10050d000000     	movsd	xmm0, qword ptr [rip + 0xd]
;;      	 f20f5ac0             	cvtsd2ss	xmm0, xmm0
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3d:	 0f0b                 	ud2	
;;   3f:	 0000                 	add	byte ptr [rax], al
;;   41:	 0000                 	add	byte ptr [rax], al
;;   43:	 0000                 	add	byte ptr [rax], al
;;   45:	 00f0                 	add	al, dh
