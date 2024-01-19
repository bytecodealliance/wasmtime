;;! target = "x86_64"

(module
  (func (export "select-i64") (param i64 i64 i32) (result i64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec20             	sub	rsp, 0x20
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8730000000         	ja	0x48
;;   18:	 48897c2418           	mov	qword ptr [rsp + 0x18], rdi
;;      	 4889742410           	mov	qword ptr [rsp + 0x10], rsi
;;      	 8954240c             	mov	dword ptr [rsp + 0xc], edx
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 488b4c2410           	mov	rcx, qword ptr [rsp + 0x10]
;;      	 488b542418           	mov	rdx, qword ptr [rsp + 0x18]
;;      	 83f800               	cmp	eax, 0
;;      	 480f45ca             	cmovne	rcx, rdx
;;      	 4889c8               	mov	rax, rcx
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   48:	 0f0b                 	ud2	
