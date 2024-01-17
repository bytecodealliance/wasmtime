;;! target = "x86_64"

(module
    (func (result i32)
        (i64.const 9223372036854775807)
        (i64.eqz)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8721000000         	ja	0x39
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48b8ffffffffffffff7f 	
;; 				movabs	rax, 0x7fffffffffffffff
;;      	 4883f800             	cmp	rax, 0
;;      	 b800000000           	mov	eax, 0
;;      	 400f94c0             	sete	al
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   39:	 0f0b                 	ud2	
