;;! target = "x86_64"

(module
  (func $dummy)
  (func (export "singular") (param i32) (result i32)
    (if (local.get 0) (then (nop)))
    (if (local.get 0) (then (nop)) (else (nop)))
    (if (result i32) (local.get 0) (then (i32.const 7)) (else (i32.const 8)))
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
;;   17:	 0f8400000000         	je	0x1d
;;   1d:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   21:	 85c0                 	test	eax, eax
;;   23:	 0f8400000000         	je	0x29
;;   29:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   2d:	 85c0                 	test	eax, eax
;;   2f:	 0f840c000000         	je	0x41
;;   35:	 48c7c007000000       	mov	rax, 7
;;   3c:	 e907000000           	jmp	0x48
;;   41:	 48c7c008000000       	mov	rax, 8
;;   48:	 4883c410             	add	rsp, 0x10
;;   4c:	 5d                   	pop	rbp
;;   4d:	 c3                   	ret	
