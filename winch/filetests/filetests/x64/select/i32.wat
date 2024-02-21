;;! target = "x86_64"

(module
  (func (export "select-i32") (param i32 i32 i32) (result i32)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8735000000         	ja	0x53
;;   1e:	 4883ec20             	sub	rsp, 0x20
;;      	 48897c2418           	mov	qword ptr [rsp + 0x18], rdi
;;      	 4889742410           	mov	qword ptr [rsp + 0x10], rsi
;;      	 8954240c             	mov	dword ptr [rsp + 0xc], edx
;;      	 894c2408             	mov	dword ptr [rsp + 8], ecx
;;      	 4489442404           	mov	dword ptr [rsp + 4], r8d
;;      	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;      	 8b4c2408             	mov	ecx, dword ptr [rsp + 8]
;;      	 8b54240c             	mov	edx, dword ptr [rsp + 0xc]
;;      	 83f800               	cmp	eax, 0
;;      	 0f45ca               	cmovne	ecx, edx
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   53:	 0f0b                 	ud2	
