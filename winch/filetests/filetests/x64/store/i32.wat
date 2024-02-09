;;! target = "x86_64"
(module
  (memory 1)

  (func (export "as-block-value")
    (block (i32.store (i32.const 0) (i32.const 1)))
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f871d000000         	ja	0x35
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b801000000           	mov	eax, 1
;;      	 b900000000           	mov	ecx, 0
;;      	 498b5650             	mov	rdx, qword ptr [r14 + 0x50]
;;      	 4801ca               	add	rdx, rcx
;;      	 8902                 	mov	dword ptr [rdx], eax
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   35:	 0f0b                 	ud2	
