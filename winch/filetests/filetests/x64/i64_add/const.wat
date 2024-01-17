;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 10)
	(i64.const 20)
	(i64.add)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8715000000         	ja	0x2d
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c00a000000       	mov	rax, 0xa
;;      	 4883c014             	add	rax, 0x14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   2d:	 0f0b                 	ud2	
