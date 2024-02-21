;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 0x80000000)
	(i32.const -1)
	(i32.div_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8729000000         	ja	0x47
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 b9ffffffff           	mov	ecx, 0xffffffff
;;      	 b800000080           	mov	eax, 0x80000000
;;      	 83f900               	cmp	ecx, 0
;;      	 0f840b000000         	je	0x49
;;   3e:	 99                   	cdq	
;;      	 f7f9                 	idiv	ecx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   47:	 0f0b                 	ud2	
;;   49:	 0f0b                 	ud2	
