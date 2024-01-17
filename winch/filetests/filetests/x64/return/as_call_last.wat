;;! target = "x86_64"
(module
  (func $f (param i32 i32 i32) (result i32) (i32.const -1))
  (func (export "as-call-last") (result i32)
    (call $f (i32.const 1) (i32.const 2) (return (i32.const 14)))
  )
)
  
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec18             	sub	rsp, 0x18
;;      	 897c2414             	mov	dword ptr [rsp + 0x14], edi
;;      	 89742410             	mov	dword ptr [rsp + 0x10], esi
;;      	 8954240c             	mov	dword ptr [rsp + 0xc], edx
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b8ffffffff           	mov	eax, 0xffffffff
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b80e000000           	mov	eax, 0xe
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
