;;! target = "x86_64"

(module
  (func (export "select-i64") (param i64 i64 i32) (result i64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec20             	sub	rsp, 0x20
;;    8:	 48897c2418           	mov	qword ptr [rsp + 0x18], rdi
;;    d:	 4889742410           	mov	qword ptr [rsp + 0x10], rsi
;;   12:	 8954240c             	mov	dword ptr [rsp + 0xc], edx
;;   16:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   1b:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   1f:	 488b4c2410           	mov	rcx, qword ptr [rsp + 0x10]
;;   24:	 488b542418           	mov	rdx, qword ptr [rsp + 0x18]
;;   29:	 85c0                 	test	eax, eax
;;   2b:	 0f8403000000         	je	0x34
;;   31:	 4889d1               	mov	rcx, rdx
;;   34:	 4889c8               	mov	rax, rcx
;;   37:	 4883c420             	add	rsp, 0x20
;;   3b:	 5d                   	pop	rbp
;;   3c:	 c3                   	ret	
