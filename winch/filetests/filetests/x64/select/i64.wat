;;! target = "x86_64"

(module
  (func (export "select-i64") (param i64 i64 i32) (result i64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c328000000       	add	r11, 0x28
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f873b000000         	ja	0x59
;;   1e:	 4883ec28             	sub	rsp, 0x28
;;      	 48897c2420           	mov	qword ptr [rsp + 0x20], rdi
;;      	 4889742418           	mov	qword ptr [rsp + 0x18], rsi
;;      	 4889542410           	mov	qword ptr [rsp + 0x10], rdx
;;      	 48894c2408           	mov	qword ptr [rsp + 8], rcx
;;      	 4489442404           	mov	dword ptr [rsp + 4], r8d
;;      	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;      	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;      	 488b542410           	mov	rdx, qword ptr [rsp + 0x10]
;;      	 83f800               	cmp	eax, 0
;;      	 480f45ca             	cmovne	rcx, rdx
;;      	 4889c8               	mov	rax, rcx
;;      	 4883c428             	add	rsp, 0x28
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   59:	 0f0b                 	ud2	
