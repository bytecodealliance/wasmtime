;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "nested") (param i32 i32) (result i32)
    (if (result i32) (local.get 0)
      (then
        (if (local.get 1) (then (call $dummy) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 9))
          (else (call $dummy) (i32.const 10))
        )
      )
      (else
        (if (local.get 1) (then (call $dummy) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 10))
          (else (call $dummy) (i32.const 11))
        )
      )
    )
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   18:	 85c0                 	test	eax, eax
;;   1a:	 0f8455000000         	je	0x75
;;   20:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   24:	 85c0                 	test	eax, eax
;;   26:	 0f8405000000         	je	0x31
;;   2c:	 e800000000           	call	0x31
;;   31:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   35:	 85c0                 	test	eax, eax
;;   37:	 0f8405000000         	je	0x42
;;   3d:	 e905000000           	jmp	0x47
;;   42:	 e800000000           	call	0x47
;;   47:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   4b:	 85c0                 	test	eax, eax
;;   4d:	 0f8411000000         	je	0x64
;;   53:	 e800000000           	call	0x58
;;   58:	 48c7c009000000       	mov	rax, 9
;;   5f:	 e961000000           	jmp	0xc5
;;   64:	 e800000000           	call	0x69
;;   69:	 48c7c00a000000       	mov	rax, 0xa
;;   70:	 e950000000           	jmp	0xc5
;;   75:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   79:	 85c0                 	test	eax, eax
;;   7b:	 0f8405000000         	je	0x86
;;   81:	 e800000000           	call	0x86
;;   86:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   8a:	 85c0                 	test	eax, eax
;;   8c:	 0f8405000000         	je	0x97
;;   92:	 e905000000           	jmp	0x9c
;;   97:	 e800000000           	call	0x9c
;;   9c:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   a0:	 85c0                 	test	eax, eax
;;   a2:	 0f8411000000         	je	0xb9
;;   a8:	 e800000000           	call	0xad
;;   ad:	 48c7c00a000000       	mov	rax, 0xa
;;   b4:	 e90c000000           	jmp	0xc5
;;   b9:	 e800000000           	call	0xbe
;;   be:	 48c7c00b000000       	mov	rax, 0xb
;;   c5:	 4883c410             	add	rsp, 0x10
;;   c9:	 5d                   	pop	rbp
;;   ca:	 c3                   	ret	
