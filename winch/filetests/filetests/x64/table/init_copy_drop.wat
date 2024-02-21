;;! target = "x86_64"

(module
  (type (func (result i32)))  ;; type #0
  (import "a" "ef0" (func (result i32)))    ;; index 0
  (import "a" "ef1" (func (result i32)))
  (import "a" "ef2" (func (result i32)))
  (import "a" "ef3" (func (result i32)))
  (import "a" "ef4" (func (result i32)))    ;; index 4
  (table $t0 30 30 funcref)
  (table $t1 30 30 funcref)
  (elem (table $t0) (i32.const 2) func 3 1 4 1)
  (elem funcref
    (ref.func 2) (ref.func 7) (ref.func 1) (ref.func 8))
  (elem (table $t0) (i32.const 12) func 7 5 2 3 6)
  (elem funcref
    (ref.func 5) (ref.func 9) (ref.func 2) (ref.func 7) (ref.func 6))
  (func (result i32) (i32.const 5))  ;; index 5
  (func (result i32) (i32.const 6))
  (func (result i32) (i32.const 7))
  (func (result i32) (i32.const 8))
  (func (result i32) (i32.const 9))  ;; index 9
  (func (export "test")
    (table.init $t0 1 (i32.const 7) (i32.const 0) (i32.const 4))
         (elem.drop 1)
         (table.init $t0 3 (i32.const 15) (i32.const 1) (i32.const 3))
         (elem.drop 3)
         (table.copy $t0 0 (i32.const 20) (i32.const 15) (i32.const 5))
         (table.copy $t0 0 (i32.const 21) (i32.const 29) (i32.const 1))
         (table.copy $t0 0 (i32.const 24) (i32.const 10) (i32.const 1))
         (table.copy $t0 0 (i32.const 13) (i32.const 11) (i32.const 4))
         (table.copy $t0 0 (i32.const 19) (i32.const 20) (i32.const 5)))
  (func (export "check") (param i32) (result i32)
    (call_indirect $t0 (type 0) (local.get 0)))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8718000000         	ja	0x36
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 b805000000           	mov	eax, 5
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8718000000         	ja	0x36
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 b806000000           	mov	eax, 6
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8718000000         	ja	0x36
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 b807000000           	mov	eax, 7
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8718000000         	ja	0x36
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 b808000000           	mov	eax, 8
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8718000000         	ja	0x36
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 b809000000           	mov	eax, 9
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   36:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c310000000       	add	r11, 0x10
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f877c010000         	ja	0x19a
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4310             	mov	rax, qword ptr [r11 + 0x10]
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 ba01000000           	mov	edx, 1
;;      	 b907000000           	mov	ecx, 7
;;      	 41b800000000         	mov	r8d, 0
;;      	 41b904000000         	mov	r9d, 4
;;      	 ffd0                 	call	rax
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4318             	mov	rax, qword ptr [r11 + 0x18]
;;      	 4c89f7               	mov	rdi, r14
;;      	 be01000000           	mov	esi, 1
;;      	 ffd0                 	call	rax
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4310             	mov	rax, qword ptr [r11 + 0x10]
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 ba03000000           	mov	edx, 3
;;      	 b90f000000           	mov	ecx, 0xf
;;      	 41b801000000         	mov	r8d, 1
;;      	 41b903000000         	mov	r9d, 3
;;      	 ffd0                 	call	rax
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4318             	mov	rax, qword ptr [r11 + 0x18]
;;      	 4c89f7               	mov	rdi, r14
;;      	 be03000000           	mov	esi, 3
;;      	 ffd0                 	call	rax
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 b914000000           	mov	ecx, 0x14
;;      	 41b80f000000         	mov	r8d, 0xf
;;      	 41b905000000         	mov	r9d, 5
;;      	 ffd0                 	call	rax
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 b915000000           	mov	ecx, 0x15
;;      	 41b81d000000         	mov	r8d, 0x1d
;;      	 41b901000000         	mov	r9d, 1
;;      	 ffd0                 	call	rax
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 b918000000           	mov	ecx, 0x18
;;      	 41b80a000000         	mov	r8d, 0xa
;;      	 41b901000000         	mov	r9d, 1
;;      	 ffd0                 	call	rax
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 b90d000000           	mov	ecx, 0xd
;;      	 41b80b000000         	mov	r8d, 0xb
;;      	 41b904000000         	mov	r9d, 4
;;      	 ffd0                 	call	rax
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 b913000000           	mov	ecx, 0x13
;;      	 41b814000000         	mov	r8d, 0x14
;;      	 41b905000000         	mov	r9d, 5
;;      	 ffd0                 	call	rax
;;      	 4c8b742408           	mov	r14, qword ptr [rsp + 8]
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;  19a:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c320000000       	add	r11, 0x20
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87d4000000         	ja	0xf2
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 89542404             	mov	dword ptr [rsp + 4], edx
;;      	 448b5c2404           	mov	r11d, dword ptr [rsp + 4]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 4c89f2               	mov	rdx, r14
;;      	 8b9af0000000         	mov	ebx, dword ptr [rdx + 0xf0]
;;      	 39d9                 	cmp	ecx, ebx
;;      	 0f839f000000         	jae	0xf4
;;   55:	 4189cb               	mov	r11d, ecx
;;      	 4d6bdb08             	imul	r11, r11, 8
;;      	 488b92e8000000       	mov	rdx, qword ptr [rdx + 0xe8]
;;      	 4889d6               	mov	rsi, rdx
;;      	 4c01da               	add	rdx, r11
;;      	 39d9                 	cmp	ecx, ebx
;;      	 480f43d6             	cmovae	rdx, rsi
;;      	 488b02               	mov	rax, qword ptr [rdx]
;;      	 4885c0               	test	rax, rax
;;      	 0f8533000000         	jne	0xae
;;   7b:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 4883ec04             	sub	rsp, 4
;;      	 4c89f7               	mov	rdi, r14
;;      	 be00000000           	mov	esi, 0
;;      	 8b542404             	mov	edx, dword ptr [rsp + 4]
;;      	 ffd3                 	call	rbx
;;      	 4883c404             	add	rsp, 4
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742410           	mov	r14, qword ptr [rsp + 0x10]
;;      	 e904000000           	jmp	0xb2
;;   ae:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;      	 4885c0               	test	rax, rax
;;      	 0f843b000000         	je	0xf6
;;   bb:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;      	 418b0b               	mov	ecx, dword ptr [r11]
;;      	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;      	 39d1                 	cmp	ecx, edx
;;      	 0f852b000000         	jne	0xf8
;;   cd:	 50                   	push	rax
;;      	 59                   	pop	rcx
;;      	 488b5920             	mov	rbx, qword ptr [rcx + 0x20]
;;      	 488b5110             	mov	rdx, qword ptr [rcx + 0x10]
;;      	 4883ec08             	sub	rsp, 8
;;      	 4889df               	mov	rdi, rbx
;;      	 4c89f6               	mov	rsi, r14
;;      	 ffd2                 	call	rdx
;;      	 4883c408             	add	rsp, 8
;;      	 4c8b742410           	mov	r14, qword ptr [rsp + 0x10]
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   f2:	 0f0b                 	ud2	
;;   f4:	 0f0b                 	ud2	
;;   f6:	 0f0b                 	ud2	
;;   f8:	 0f0b                 	ud2	
