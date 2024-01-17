;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-test-operand") (param i32) (result i32)
    (i32.eqz
      (if (result i32) (local.get 0)
        (then (call $dummy) (i32.const 13))
        (else (call $dummy) (i32.const 0))
      )
    )
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 85c0                 	test	eax, eax
;;      	 0f840f000000         	je	0x2b
;;   1c:	 e800000000           	call	0x21
;;      	 b80d000000           	mov	eax, 0xd
;;      	 e90a000000           	jmp	0x35
;;   2b:	 e800000000           	call	0x30
;;      	 b800000000           	mov	eax, 0
;;      	 83f800               	cmp	eax, 0
;;      	 b800000000           	mov	eax, 0
;;      	 400f94c0             	sete	al
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
