;;! target = "x86_64"

(module
  (memory (data "\00\00\00\00\00\00\f4\7f"))
  (func (export "f64.store") (f64.store (i32.const 0) (f64.const nan:0x4000000000000)))
)

;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8726000000         	ja	0x41
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10051d000000     	movsd	xmm0, qword ptr [rip + 0x1d]
;;      	 b800000000           	mov	eax, 0
;;      	 498b4e50             	mov	rcx, qword ptr [r14 + 0x50]
;;      	 4801c1               	add	rcx, rax
;;      	 f20f1101             	movsd	qword ptr [rcx], xmm0
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   41:	 0f0b                 	ud2	
;;   43:	 0000                 	add	byte ptr [rax], al
;;   45:	 0000                 	add	byte ptr [rax], al
;;   47:	 0000                 	add	byte ptr [rax], al
;;   49:	 0000                 	add	byte ptr [rax], al
;;   4b:	 0000                 	add	byte ptr [rax], al
;;   4d:	 00f4                 	add	ah, dh
