;;! target = "x86_64"

(module
  (func (export "") 
    call 1
    call 1
    br_if 0
    drop
  )
  (func (;1;) (result i32)
    i32.const 1
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8744000000         	ja	0x5f
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883ec08             	sub	rsp, 8
;;      	 e800000000           	call	0x2c
;;      	 4883c408             	add	rsp, 8
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4883ec04             	sub	rsp, 4
;;      	 e800000000           	call	0x40
;;      	 4883c404             	add	rsp, 4
;;      	 85c0                 	test	eax, eax
;;      	 0f8409000000         	je	0x55
;;   4c:	 4883c404             	add	rsp, 4
;;      	 e904000000           	jmp	0x59
;;   55:	 4883c404             	add	rsp, 4
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5f:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8713000000         	ja	0x2e
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b801000000           	mov	eax, 1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   2e:	 0f0b                 	ud2	
