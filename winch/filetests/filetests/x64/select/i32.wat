;;! target = "x86_64"

(module
  (func (export "select-i32") (param i32 i32 i32) (result i32)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 897c2414             	mov	dword ptr [rsp + 0x14], edi
;;    c:	 89742410             	mov	dword ptr [rsp + 0x10], esi
;;   10:	 8954240c             	mov	dword ptr [rsp + 0xc], edx
;;   14:	 4c893424             	mov	qword ptr [rsp], r14
;;   18:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   1c:	 8b4c2410             	mov	ecx, dword ptr [rsp + 0x10]
;;   20:	 8b542414             	mov	edx, dword ptr [rsp + 0x14]
;;   24:	 83f800               	cmp	eax, 0
;;   27:	 0f45ca               	cmovne	ecx, edx
;;   2a:	 89c8                 	mov	eax, ecx
;;   2c:	 4883c418             	add	rsp, 0x18
;;   30:	 5d                   	pop	rbp
;;   31:	 c3                   	ret	
