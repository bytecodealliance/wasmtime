;;! target = "x86_64"

(module
  (func $dummy)
  (func (export "as-if-condition") (param i32) (result i32)
    (if (result i32)
      (if (result i32) (local.get 0)
        (then (i32.const 1)) (else (i32.const 0))
      )
      (then (call $dummy) (i32.const 2))
      (else (call $dummy) (i32.const 3))
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
;;   17:	 0f840c000000         	je	0x29
;;   1d:	 48c7c001000000       	mov	rax, 1
;;   24:	 e907000000           	jmp	0x30
;;   29:	 48c7c000000000       	mov	rax, 0
;;   30:	 85c0                 	test	eax, eax
;;   32:	 0f8411000000         	je	0x49
;;   38:	 e800000000           	call	0x3d
;;   3d:	 48c7c002000000       	mov	rax, 2
;;   44:	 e90c000000           	jmp	0x55
;;   49:	 e800000000           	call	0x4e
;;   4e:	 48c7c003000000       	mov	rax, 3
;;   55:	 4883c410             	add	rsp, 0x10
;;   59:	 5d                   	pop	rbp
;;   5a:	 c3                   	ret	
