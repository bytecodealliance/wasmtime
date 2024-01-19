;;! target = "x86_64"
(module
  (func (export "as-br-value") (result i32)
    (block (result i32) (br 0 (br 0 (i32.const 9))))
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870f000000         	ja	0x27
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b809000000           	mov	eax, 9
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   27:	 0f0b                 	ud2	
