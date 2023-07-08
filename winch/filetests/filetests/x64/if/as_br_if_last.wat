;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-br_if-last") (param i32) (result i32)
    (block (result i32)
      (br_if 0
        (i32.const 2)
        (if (result i32) (local.get 0)
          (then (call $dummy) (i32.const 1))
          (else (call $dummy) (i32.const 0))
        )
      )
      (return (i32.const 3))
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
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   15:	 85c0                 	test	eax, eax
;;   17:	 0f8411000000         	je	0x2e
;;   1d:	 e800000000           	call	0x22
;;   22:	 48c7c001000000       	mov	rax, 1
;;   29:	 e90c000000           	jmp	0x3a
;;   2e:	 e800000000           	call	0x33
;;   33:	 48c7c000000000       	mov	rax, 0
;;   3a:	 50                   	push	rax
;;   3b:	 59                   	pop	rcx
;;   3c:	 48c7c002000000       	mov	rax, 2
;;   43:	 85c9                 	test	ecx, ecx
;;   45:	 0f850c000000         	jne	0x57
;;   4b:	 50                   	push	rax
;;   4c:	 48c7c003000000       	mov	rax, 3
;;   53:	 4883c408             	add	rsp, 8
;;   57:	 4883c410             	add	rsp, 0x10
;;   5b:	 5d                   	pop	rbp
;;   5c:	 c3                   	ret	
