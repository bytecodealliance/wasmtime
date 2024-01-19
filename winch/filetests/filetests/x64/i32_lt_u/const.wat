;;! target = "x86_64"

(module
    (func (result i32)
        (i32.const 2)
        (i32.const 3)
        (i32.lt_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f871b000000         	ja	0x33
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b802000000           	mov	eax, 2
;;      	 83f803               	cmp	eax, 3
;;      	 b800000000           	mov	eax, 0
;;      	 400f92c0             	setb	al
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   33:	 0f0b                 	ud2	
