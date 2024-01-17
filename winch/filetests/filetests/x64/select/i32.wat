;;! target = "x86_64"

(module
  (func (export "select-i32") (param i32 i32 i32) (result i32)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec18             	sub	rsp, 0x18
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872a000000         	ja	0x42
;;   18:	 897c2414             	mov	dword ptr [rsp + 0x14], edi
;;      	 89742410             	mov	dword ptr [rsp + 0x10], esi
;;      	 8954240c             	mov	dword ptr [rsp + 0xc], edx
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 8b4c2410             	mov	ecx, dword ptr [rsp + 0x10]
;;      	 8b542414             	mov	edx, dword ptr [rsp + 0x14]
;;      	 83f800               	cmp	eax, 0
;;      	 0f45ca               	cmovne	ecx, edx
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   42:	 0f0b                 	ud2	
