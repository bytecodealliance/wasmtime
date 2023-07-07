;;! target = "x86_64"

(module
  (func (export "as-if-then-no-else") (param i32 i32) (result i32)
    (if (local.get 0) (then (unreachable))) (local.get 1)
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   18:	 85c0                 	test	eax, eax
;;   1a:	 0f8402000000         	je	0x22
;;   20:	 0f0b                 	ud2	
;;   22:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   26:	 4883c410             	add	rsp, 0x10
;;   2a:	 5d                   	pop	rbp
;;   2b:	 c3                   	ret	
