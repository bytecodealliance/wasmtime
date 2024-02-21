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
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8713000000         	ja	0x31
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   31:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8788000000         	ja	0xa6
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 e800000000           	call	0x36
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 e800000000           	call	0x46
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 e800000000           	call	0x56
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 e800000000           	call	0x66
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 e800000000           	call	0x76
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 e800000000           	call	0x86
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4c89f7               	mov	rdi, r14
;;      	 4c89f6               	mov	rsi, r14
;;      	 e800000000           	call	0x96
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 b808000000           	mov	eax, 8
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   a6:	 0f0b                 	ud2	
