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
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870e000000         	ja	0x29
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   29:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f876e000000         	ja	0x89
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x2c
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x39
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x46
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x53
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x60
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x6d
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x7a
;;      	 4883c408             	add	rsp, 8
;;      	 b808000000           	mov	eax, 8
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   89:	 0f0b                 	ud2	
