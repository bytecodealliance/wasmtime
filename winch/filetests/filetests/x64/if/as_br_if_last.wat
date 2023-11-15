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
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 85c0                 	test	eax, eax
;;   16:	 0f840f000000         	je	0x2b
;;   1c:	 e800000000           	call	0x21
;;   21:	 b801000000           	mov	eax, 1
;;   26:	 e90a000000           	jmp	0x35
;;   2b:	 e800000000           	call	0x30
;;   30:	 b800000000           	mov	eax, 0
;;   35:	 4883ec04             	sub	rsp, 4
;;   39:	 890424               	mov	dword ptr [rsp], eax
;;   3c:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   3f:	 4883c404             	add	rsp, 4
;;   43:	 b802000000           	mov	eax, 2
;;   48:	 85c9                 	test	ecx, ecx
;;   4a:	 0f8510000000         	jne	0x60
;;   50:	 4883ec04             	sub	rsp, 4
;;   54:	 890424               	mov	dword ptr [rsp], eax
;;   57:	 b803000000           	mov	eax, 3
;;   5c:	 4883c404             	add	rsp, 4
;;   60:	 4883c410             	add	rsp, 0x10
;;   64:	 5d                   	pop	rbp
;;   65:	 c3                   	ret	
