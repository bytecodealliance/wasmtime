;;! target = "x86_64"

(module
  (func (export "type-i64") (result i32) (unreachable))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8715000000         	ja	0x33
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 0f0b                 	ud2	
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   33:	 0f0b                 	ud2	
