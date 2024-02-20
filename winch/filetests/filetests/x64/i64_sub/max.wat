;;! target = "x86_64"
(module
    (func (result i64)
	(i64.const 0x7fffffffffffffff)
	(i64.const -1)
	(i64.sub)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f871c000000         	ja	0x37
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48b8ffffffffffffff7f 	
;; 				movabs	rax, 0x7fffffffffffffff
;;      	 4883e8ff             	sub	rax, -1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   37:	 0f0b                 	ud2	
