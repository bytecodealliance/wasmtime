;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const -1)
	(i64.div_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872a000000         	ja	0x42
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c1ffffffff       	mov	rcx, 0xffffffffffffffff
;;      	 48b80000000000000080 	
;; 				movabs	rax, 0x8000000000000000
;;      	 4883f900             	cmp	rcx, 0
;;      	 0f840d000000         	je	0x44
;;   37:	 4899                 	cqo	
;;      	 48f7f9               	idiv	rcx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   42:	 0f0b                 	ud2	
;;   44:	 0f0b                 	ud2	
