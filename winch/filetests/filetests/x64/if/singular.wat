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
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 85c0                 	test	eax, eax
;;   16:	 0f8400000000         	je	0x1c
;;   1c:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   20:	 85c0                 	test	eax, eax
;;   22:	 0f8400000000         	je	0x28
;;   28:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   2c:	 85c0                 	test	eax, eax
;;   2e:	 0f840a000000         	je	0x3e
;;   34:	 b807000000           	mov	eax, 7
;;   39:	 e905000000           	jmp	0x43
;;   3e:	 b808000000           	mov	eax, 8
;;   43:	 4883c410             	add	rsp, 0x10
;;   47:	 5d                   	pop	rbp
;;   48:	 c3                   	ret	
