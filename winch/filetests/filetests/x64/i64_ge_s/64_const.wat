;;! target = "x86_64"

(module
    (func (result i32)
        (i64.const 9223372036854775806)
        (i64.const 9223372036854775807)
        (i64.ge_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8733000000         	ja	0x51
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 48b8feffffffffffff7f 	
;; 				movabs	rax, 0x7ffffffffffffffe
;;      	 49bbffffffffffffff7f 	
;; 				movabs	r11, 0x7fffffffffffffff
;;      	 4c39d8               	cmp	rax, r11
;;      	 b800000000           	mov	eax, 0
;;      	 400f9dc0             	setge	al
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   51:	 0f0b                 	ud2	
