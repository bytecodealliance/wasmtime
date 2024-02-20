;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const 20)
	(i32.const 10)
	(i32.div_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8724000000         	ja	0x3f
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b90a000000           	mov	ecx, 0xa
;;      	 b814000000           	mov	eax, 0x14
;;      	 83f900               	cmp	ecx, 0
;;      	 0f840b000000         	je	0x41
;;   36:	 99                   	cdq	
;;      	 f7f9                 	idiv	ecx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3f:	 0f0b                 	ud2	
;;   41:	 0f0b                 	ud2	
