;;! target = "x86_64"
(module
  (func $f (param i32 i32 i32) (result i32) (i32.const -1))
  (func (export "as-call-last") (result i32)
    (block (result i32)
      (call $f
        (i32.const 1) (i32.const 2) (br_if 0 (i32.const 14) (i32.const 1))
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
;;   11:	 b80e000000           	mov	eax, 0xe
;;   16:	 85c9                 	test	ecx, ecx
;;   18:	 0f8526000000         	jne	0x44
;;   1e:	 4883ec04             	sub	rsp, 4
;;   22:	 890424               	mov	dword ptr [rsp], eax
;;   25:	 4883ec04             	sub	rsp, 4
;;   29:	 bf01000000           	mov	edi, 1
;;   2e:	 be02000000           	mov	esi, 2
;;   33:	 8b542404             	mov	edx, dword ptr [rsp + 4]
;;   37:	 e800000000           	call	0x3c
;;   3c:	 4883c404             	add	rsp, 4
;;   40:	 4883c404             	add	rsp, 4
;;   44:	 4883c408             	add	rsp, 8
;;   48:	 5d                   	pop	rbp
;;   49:	 c3                   	ret	
