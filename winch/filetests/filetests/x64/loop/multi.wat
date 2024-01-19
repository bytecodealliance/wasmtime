;;! target = "x86_64"

(module
  (func $dummy)
  (func (export "multi") (result i32)
    (loop (call $dummy) (call $dummy) (call $dummy) (call $dummy))
    (loop (result i32) (call $dummy) (call $dummy) (i32.const 8) (call $dummy))
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870a000000         	ja	0x22
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   22:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f876a000000         	ja	0x82
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x25
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x32
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x3f
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x4c
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x59
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x66
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x73
;;      	 4883c408             	add	rsp, 8
;;      	 b808000000           	mov	eax, 8
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   82:	 0f0b                 	ud2	
