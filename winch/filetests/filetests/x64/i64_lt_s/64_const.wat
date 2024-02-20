;;! target = "x86_64"

(module
    (func (result i32)
        (i64.const 9223372036854775806)
        (i64.const 9223372036854775807)
        (i64.lt_s)
    )
)

;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872e000000         	ja	0x49
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48b8feffffffffffff7f 	
;; 				movabs	rax, 0x7ffffffffffffffe
;;      	 49bbffffffffffffff7f 	
;; 				movabs	r11, 0x7fffffffffffffff
;;      	 4c39d8               	cmp	rax, r11
;;      	 b800000000           	mov	eax, 0
;;      	 400f9cc0             	setl	al
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   49:	 0f0b                 	ud2	
