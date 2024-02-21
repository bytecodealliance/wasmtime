;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 7)
	(i32.const 5)
	(i32.rem_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8723000000         	ja	0x41
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 b905000000           	mov	ecx, 5
;;      	 b807000000           	mov	eax, 7
;;      	 31d2                 	xor	edx, edx
;;      	 f7f1                 	div	ecx
;;      	 89d0                 	mov	eax, edx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   41:	 0f0b                 	ud2	
