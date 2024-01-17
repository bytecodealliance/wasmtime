;;! target = "x86_64"
(module
  (func (export "as-br-if-cond")
    (block (br_if 0 (br_if 0 (i32.const 1) (i32.const 1))))
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8724000000         	ja	0x3c
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b801000000           	mov	eax, 1
;;      	 85c0                 	test	eax, eax
;;      	 0f850d000000         	jne	0x36
;;   29:	 b801000000           	mov	eax, 1
;;      	 85c0                 	test	eax, eax
;;      	 0f8500000000         	jne	0x36
;;   36:	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3c:	 0f0b                 	ud2	
