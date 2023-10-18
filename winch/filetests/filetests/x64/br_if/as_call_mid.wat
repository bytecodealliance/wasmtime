;;! target = "x86_64"
(module
  (func $f (param i32 i32 i32) (result i32) (i32.const -1)) 
  (func (export "as-call-mid") (result i32)
    (block (result i32)
      (call $f
        (i32.const 1) (br_if 0 (i32.const 13) (i32.const 1)) (i32.const 3)
      )
    )
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 897c2414             	mov	dword ptr [rsp + 0x14], edi
;;    c:	 89742410             	mov	dword ptr [rsp + 0x10], esi
;;   10:	 8954240c             	mov	dword ptr [rsp + 0xc], edx
;;   14:	 4c893424             	mov	qword ptr [rsp], r14
;;   18:	 b8ffffffff           	mov	eax, 0xffffffff
;;   1d:	 4883c418             	add	rsp, 0x18
;;   21:	 5d                   	pop	rbp
;;   22:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b901000000           	mov	ecx, 1
;;   11:	 b80d000000           	mov	eax, 0xd
;;   16:	 85c9                 	test	ecx, ecx
;;   18:	 0f8517000000         	jne	0x35
;;   1e:	 50                   	push	rax
;;   1f:	 bf01000000           	mov	edi, 1
;;   24:	 8b3424               	mov	esi, dword ptr [rsp]
;;   27:	 ba03000000           	mov	edx, 3
;;   2c:	 e800000000           	call	0x31
;;   31:	 4883c408             	add	rsp, 8
;;   35:	 4883c408             	add	rsp, 8
;;   39:	 5d                   	pop	rbp
;;   3a:	 c3                   	ret	
