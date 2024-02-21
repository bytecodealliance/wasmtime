;;! target = "x86_64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.rem_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c318000000       	add	r11, 0x18
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f873a000000         	ja	0x58
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 89542404             	mov	dword ptr [rsp + 4], edx
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;      	 99                   	cdq	
;;      	 83f9ff               	cmp	ecx, -1
;;      	 0f850a000000         	jne	0x4e
;;   44:	 ba00000000           	mov	edx, 0
;;      	 e902000000           	jmp	0x50
;;   4e:	 f7f9                 	idiv	ecx
;;      	 89d0                 	mov	eax, edx
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   58:	 0f0b                 	ud2	
