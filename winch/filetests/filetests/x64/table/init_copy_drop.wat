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
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870f000000         	ja	0x27
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b805000000           	mov	eax, 5
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   27:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870f000000         	ja	0x27
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b806000000           	mov	eax, 6
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   27:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870f000000         	ja	0x27
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b807000000           	mov	eax, 7
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   27:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870f000000         	ja	0x27
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b808000000           	mov	eax, 8
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   27:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f870f000000         	ja	0x27
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b809000000           	mov	eax, 9
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   27:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8785010000         	ja	0x19d
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4310             	mov	rax, qword ptr [r11 + 0x10]
;;      	 4156                 	push	r14
;;      	 488b3c24             	mov	rdi, qword ptr [rsp]
;;      	 be00000000           	mov	esi, 0
;;      	 ba01000000           	mov	edx, 1
;;      	 b907000000           	mov	ecx, 7
;;      	 41b800000000         	mov	r8d, 0
;;      	 41b904000000         	mov	r9d, 4
;;      	 ffd0                 	call	rax
;;      	 4883c408             	add	rsp, 8
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4318             	mov	rax, qword ptr [r11 + 0x18]
;;      	 4156                 	push	r14
;;      	 488b3c24             	mov	rdi, qword ptr [rsp]
;;      	 be01000000           	mov	esi, 1
;;      	 ffd0                 	call	rax
;;      	 4883c408             	add	rsp, 8
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4310             	mov	rax, qword ptr [r11 + 0x10]
;;      	 4156                 	push	r14
;;      	 488b3c24             	mov	rdi, qword ptr [rsp]
;;      	 be00000000           	mov	esi, 0
;;      	 ba03000000           	mov	edx, 3
;;      	 b90f000000           	mov	ecx, 0xf
;;      	 41b801000000         	mov	r8d, 1
;;      	 41b903000000         	mov	r9d, 3
;;      	 ffd0                 	call	rax
;;      	 4883c408             	add	rsp, 8
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4318             	mov	rax, qword ptr [r11 + 0x18]
;;      	 4156                 	push	r14
;;      	 488b3c24             	mov	rdi, qword ptr [rsp]
;;      	 be03000000           	mov	esi, 3
;;      	 ffd0                 	call	rax
;;      	 4883c408             	add	rsp, 8
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;      	 4156                 	push	r14
;;      	 488b3c24             	mov	rdi, qword ptr [rsp]
;;      	 be00000000           	mov	esi, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 b914000000           	mov	ecx, 0x14
;;      	 41b80f000000         	mov	r8d, 0xf
;;      	 41b905000000         	mov	r9d, 5
;;      	 ffd0                 	call	rax
;;      	 4883c408             	add	rsp, 8
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;      	 4156                 	push	r14
;;      	 488b3c24             	mov	rdi, qword ptr [rsp]
;;      	 be00000000           	mov	esi, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 b915000000           	mov	ecx, 0x15
;;      	 41b81d000000         	mov	r8d, 0x1d
;;      	 41b901000000         	mov	r9d, 1
;;      	 ffd0                 	call	rax
;;      	 4883c408             	add	rsp, 8
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;      	 4156                 	push	r14
;;      	 488b3c24             	mov	rdi, qword ptr [rsp]
;;      	 be00000000           	mov	esi, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 b918000000           	mov	ecx, 0x18
;;      	 41b80a000000         	mov	r8d, 0xa
;;      	 41b901000000         	mov	r9d, 1
;;      	 ffd0                 	call	rax
;;      	 4883c408             	add	rsp, 8
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;      	 4156                 	push	r14
;;      	 488b3c24             	mov	rdi, qword ptr [rsp]
;;      	 be00000000           	mov	esi, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 b90d000000           	mov	ecx, 0xd
;;      	 41b80b000000         	mov	r8d, 0xb
;;      	 41b904000000         	mov	r9d, 4
;;      	 ffd0                 	call	rax
;;      	 4883c408             	add	rsp, 8
;;      	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;      	 4156                 	push	r14
;;      	 488b3c24             	mov	rdi, qword ptr [rsp]
;;      	 be00000000           	mov	esi, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 b913000000           	mov	ecx, 0x13
;;      	 41b814000000         	mov	r8d, 0x14
;;      	 41b905000000         	mov	r9d, 5
;;      	 ffd0                 	call	rax
;;      	 4883c408             	add	rsp, 8
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;  19d:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87b2000000         	ja	0xca
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 4c89f2               	mov	rdx, r14
;;      	 8b9af0000000         	mov	ebx, dword ptr [rdx + 0xf0]
;;      	 39d9                 	cmp	ecx, ebx
;;      	 0f8387000000         	jae	0xcc
;;   45:	 4189cb               	mov	r11d, ecx
;;      	 4d6bdb08             	imul	r11, r11, 8
;;      	 488b92e8000000       	mov	rdx, qword ptr [rdx + 0xe8]
;;      	 4889d6               	mov	rsi, rdx
;;      	 4c01da               	add	rdx, r11
;;      	 39d9                 	cmp	ecx, ebx
;;      	 480f43d6             	cmovae	rdx, rsi
;;      	 488b02               	mov	rax, qword ptr [rdx]
;;      	 4885c0               	test	rax, rax
;;      	 0f8532000000         	jne	0x9d
;;   6b:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;      	 498b5b48             	mov	rbx, qword ptr [r11 + 0x48]
;;      	 4156                 	push	r14
;;      	 4883ec04             	sub	rsp, 4
;;      	 890c24               	mov	dword ptr [rsp], ecx
;;      	 4883ec04             	sub	rsp, 4
;;      	 488b7c2408           	mov	rdi, qword ptr [rsp + 8]
;;      	 be00000000           	mov	esi, 0
;;      	 8b542404             	mov	edx, dword ptr [rsp + 4]
;;      	 ffd3                 	call	rbx
;;      	 4883c404             	add	rsp, 4
;;      	 4883c40c             	add	rsp, 0xc
;;      	 e904000000           	jmp	0xa1
;;   9d:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;      	 4885c0               	test	rax, rax
;;      	 0f8424000000         	je	0xce
;;   aa:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;      	 418b0b               	mov	ecx, dword ptr [r11]
;;      	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;      	 39d1                 	cmp	ecx, edx
;;      	 0f8514000000         	jne	0xd0
;;   bc:	 50                   	push	rax
;;      	 59                   	pop	rcx
;;      	 488b5110             	mov	rdx, qword ptr [rcx + 0x10]
;;      	 ffd2                 	call	rdx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   ca:	 0f0b                 	ud2	
;;   cc:	 0f0b                 	ud2	
;;   ce:	 0f0b                 	ud2	
;;   d0:	 0f0b                 	ud2	
