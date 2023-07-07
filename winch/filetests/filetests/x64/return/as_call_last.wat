;;! target = "x86_64"
(module
  (func $f (param i32 i32 i32) (result i32) (i32.const -1))
  (func (export "as-call-last") (result i32)
    (call $f (i32.const 1) (i32.const 2) (return (i32.const 14)))
  )
)
  
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 897c2414             	mov	dword ptr [rsp + 0x14], edi
;;    c:	 89742410             	mov	dword ptr [rsp + 0x10], esi
;;   10:	 8954240c             	mov	dword ptr [rsp + 0xc], edx
;;   14:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   19:	 48c7c0ffffffff       	mov	rax, 0xffffffffffffffff
;;   20:	 4883c418             	add	rsp, 0x18
;;   24:	 5d                   	pop	rbp
;;   25:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c00e000000       	mov	rax, 0xe
;;   13:	 4883c408             	add	rsp, 8
;;   17:	 5d                   	pop	rbp
;;   18:	 c3                   	ret	
