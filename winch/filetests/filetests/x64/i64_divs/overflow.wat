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
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8733000000         	ja	0x51
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 48c7c1ffffffff       	mov	rcx, 0xffffffffffffffff
;;      	 48b80000000000000080 	
;; 				movabs	rax, 0x8000000000000000
;;      	 4883f900             	cmp	rcx, 0
;;      	 0f840d000000         	je	0x53
;;   46:	 4899                 	cqo	
;;      	 48f7f9               	idiv	rcx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   51:	 0f0b                 	ud2	
;;   53:	 0f0b                 	ud2	
