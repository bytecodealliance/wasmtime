;;! target = "x86_64"
(module
  (func $f (param i32) (result i32) (local.get 0))
  (func (export "as-call-value") (result i32)
    (call $f (loop (result i32) (i32.const 1)))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   15:	 4883c410             	add	rsp, 0x10
;;   19:	 5d                   	pop	rbp
;;   1a:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883ec08             	sub	rsp, 8
;;   10:	 bf01000000           	mov	edi, 1
;;   15:	 e800000000           	call	0x1a
;;   1a:	 4883c408             	add	rsp, 8
;;   1e:	 4883c408             	add	rsp, 8
;;   22:	 5d                   	pop	rbp
;;   23:	 c3                   	ret	
