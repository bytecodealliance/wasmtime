;;! target = "x86_64"
(module
  (table $t1 0 funcref)
  (func (export "size") (result i32)
    (table.size $t1))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8715000000         	ja	0x30
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4d89f3               	mov	r11, r14
;;      	 418b4350             	mov	eax, dword ptr [r11 + 0x50]
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   30:	 0f0b                 	ud2	
