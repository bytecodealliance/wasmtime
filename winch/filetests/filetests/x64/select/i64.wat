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
;;   16:	 4c893424             	mov	qword ptr [rsp], r14
;;   1a:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   1e:	 488b4c2410           	mov	rcx, qword ptr [rsp + 0x10]
;;   23:	 488b542418           	mov	rdx, qword ptr [rsp + 0x18]
;;   28:	 83f800               	cmp	eax, 0
;;   2b:	 480f45ca             	cmovne	rcx, rdx
;;   2f:	 4889c8               	mov	rax, rcx
;;   32:	 4883c420             	add	rsp, 0x20
;;   36:	 5d                   	pop	rbp
;;   37:	 c3                   	ret	
