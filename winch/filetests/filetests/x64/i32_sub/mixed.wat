;;! target = "x86_64"

(module
    (func (result i32)
        (i32.const -1)
	(i32.const 1)
	(i32.sub)
     )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8712000000         	ja	0x2a
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b8ffffffff           	mov	eax, 0xffffffff
;;      	 83e801               	sub	eax, 1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   2a:	 0f0b                 	ud2	
