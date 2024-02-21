;;! target = "x86_64"

(module
  (memory (data "\00\00\00\00\00\00\f4\7f"))
  (func (export "f64.store") (f64.store (i32.const 0) (f64.const nan:0x4000000000000)))
)

;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872b000000         	ja	0x49
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 f20f10051d000000     	movsd	xmm0, qword ptr [rip + 0x1d]
;;      	 b800000000           	mov	eax, 0
;;      	 498b4e50             	mov	rcx, qword ptr [r14 + 0x50]
;;      	 4801c1               	add	rcx, rax
;;      	 f20f1101             	movsd	qword ptr [rcx], xmm0
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   49:	 0f0b                 	ud2	
;;   4b:	 0000                 	add	byte ptr [rax], al
;;   4d:	 0000                 	add	byte ptr [rax], al
;;   4f:	 0000                 	add	byte ptr [rax], al
;;   51:	 0000                 	add	byte ptr [rax], al
;;   53:	 0000                 	add	byte ptr [rax], al
;;   55:	 00f4                 	add	ah, dh
