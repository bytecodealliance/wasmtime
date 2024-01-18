;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 0x80000000)
	(i32.const -1)
	(i32.rem_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872c000000         	ja	0x44
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b9ffffffff           	mov	ecx, 0xffffffff
;;      	 b800000080           	mov	eax, 0x80000000
;;      	 99                   	cdq	
;;      	 83f9ff               	cmp	ecx, -1
;;      	 0f850a000000         	jne	0x3a
;;   30:	 ba00000000           	mov	edx, 0
;;      	 e902000000           	jmp	0x3c
;;   3a:	 f7f9                 	idiv	ecx
;;      	 89d0                 	mov	eax, edx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   44:	 0f0b                 	ud2	
