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
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b805000000           	mov	eax, 5
;;   11:	 4883c408             	add	rsp, 8
;;   15:	 5d                   	pop	rbp
;;   16:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b806000000           	mov	eax, 6
;;   11:	 4883c408             	add	rsp, 8
;;   15:	 5d                   	pop	rbp
;;   16:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b807000000           	mov	eax, 7
;;   11:	 4883c408             	add	rsp, 8
;;   15:	 5d                   	pop	rbp
;;   16:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b808000000           	mov	eax, 8
;;   11:	 4883c408             	add	rsp, 8
;;   15:	 5d                   	pop	rbp
;;   16:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b809000000           	mov	eax, 9
;;   11:	 4883c408             	add	rsp, 8
;;   15:	 5d                   	pop	rbp
;;   16:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   10:	 498b4310             	mov	rax, qword ptr [r11 + 0x10]
;;   14:	 4883ec08             	sub	rsp, 8
;;   18:	 4c89f7               	mov	rdi, r14
;;   1b:	 be00000000           	mov	esi, 0
;;   20:	 ba01000000           	mov	edx, 1
;;   25:	 b907000000           	mov	ecx, 7
;;   2a:	 41b800000000         	mov	r8d, 0
;;   30:	 41b904000000         	mov	r9d, 4
;;   36:	 ffd0                 	call	rax
;;   38:	 4883c408             	add	rsp, 8
;;   3c:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   40:	 498b4318             	mov	rax, qword ptr [r11 + 0x18]
;;   44:	 4883ec08             	sub	rsp, 8
;;   48:	 4c89f7               	mov	rdi, r14
;;   4b:	 be01000000           	mov	esi, 1
;;   50:	 ffd0                 	call	rax
;;   52:	 4883c408             	add	rsp, 8
;;   56:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   5a:	 498b4310             	mov	rax, qword ptr [r11 + 0x10]
;;   5e:	 4883ec08             	sub	rsp, 8
;;   62:	 4c89f7               	mov	rdi, r14
;;   65:	 be00000000           	mov	esi, 0
;;   6a:	 ba03000000           	mov	edx, 3
;;   6f:	 b90f000000           	mov	ecx, 0xf
;;   74:	 41b801000000         	mov	r8d, 1
;;   7a:	 41b903000000         	mov	r9d, 3
;;   80:	 ffd0                 	call	rax
;;   82:	 4883c408             	add	rsp, 8
;;   86:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   8a:	 498b4318             	mov	rax, qword ptr [r11 + 0x18]
;;   8e:	 4883ec08             	sub	rsp, 8
;;   92:	 4c89f7               	mov	rdi, r14
;;   95:	 be03000000           	mov	esi, 3
;;   9a:	 ffd0                 	call	rax
;;   9c:	 4883c408             	add	rsp, 8
;;   a0:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   a4:	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;   a8:	 4883ec08             	sub	rsp, 8
;;   ac:	 4c89f7               	mov	rdi, r14
;;   af:	 be00000000           	mov	esi, 0
;;   b4:	 ba00000000           	mov	edx, 0
;;   b9:	 b914000000           	mov	ecx, 0x14
;;   be:	 41b80f000000         	mov	r8d, 0xf
;;   c4:	 41b905000000         	mov	r9d, 5
;;   ca:	 ffd0                 	call	rax
;;   cc:	 4883c408             	add	rsp, 8
;;   d0:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   d4:	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;   d8:	 4883ec08             	sub	rsp, 8
;;   dc:	 4c89f7               	mov	rdi, r14
;;   df:	 be00000000           	mov	esi, 0
;;   e4:	 ba00000000           	mov	edx, 0
;;   e9:	 b915000000           	mov	ecx, 0x15
;;   ee:	 41b81d000000         	mov	r8d, 0x1d
;;   f4:	 41b901000000         	mov	r9d, 1
;;   fa:	 ffd0                 	call	rax
;;   fc:	 4883c408             	add	rsp, 8
;;  100:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;  104:	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;  108:	 4883ec08             	sub	rsp, 8
;;  10c:	 4c89f7               	mov	rdi, r14
;;  10f:	 be00000000           	mov	esi, 0
;;  114:	 ba00000000           	mov	edx, 0
;;  119:	 b918000000           	mov	ecx, 0x18
;;  11e:	 41b80a000000         	mov	r8d, 0xa
;;  124:	 41b901000000         	mov	r9d, 1
;;  12a:	 ffd0                 	call	rax
;;  12c:	 4883c408             	add	rsp, 8
;;  130:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;  134:	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;  138:	 4883ec08             	sub	rsp, 8
;;  13c:	 4c89f7               	mov	rdi, r14
;;  13f:	 be00000000           	mov	esi, 0
;;  144:	 ba00000000           	mov	edx, 0
;;  149:	 b90d000000           	mov	ecx, 0xd
;;  14e:	 41b80b000000         	mov	r8d, 0xb
;;  154:	 41b904000000         	mov	r9d, 4
;;  15a:	 ffd0                 	call	rax
;;  15c:	 4883c408             	add	rsp, 8
;;  160:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;  164:	 498b4308             	mov	rax, qword ptr [r11 + 8]
;;  168:	 4883ec08             	sub	rsp, 8
;;  16c:	 4c89f7               	mov	rdi, r14
;;  16f:	 be00000000           	mov	esi, 0
;;  174:	 ba00000000           	mov	edx, 0
;;  179:	 b913000000           	mov	ecx, 0x13
;;  17e:	 41b814000000         	mov	r8d, 0x14
;;  184:	 41b905000000         	mov	r9d, 5
;;  18a:	 ffd0                 	call	rax
;;  18c:	 4883c408             	add	rsp, 8
;;  190:	 4883c408             	add	rsp, 8
;;  194:	 5d                   	pop	rbp
;;  195:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   16:	 4153                 	push	r11
;;   18:	 4d8b5e38             	mov	r11, qword ptr [r14 + 0x38]
;;   1c:	 498b4b48             	mov	rcx, qword ptr [r11 + 0x48]
;;   20:	 5b                   	pop	rbx
;;   21:	 4d89f1               	mov	r9, r14
;;   24:	 458b91f0000000       	mov	r10d, dword ptr [r9 + 0xf0]
;;   2b:	 4439d3               	cmp	ebx, r10d
;;   2e:	 0f8366000000         	jae	0x9a
;;   34:	 4189db               	mov	r11d, ebx
;;   37:	 4d6bdb08             	imul	r11, r11, 8
;;   3b:	 4d8b89e8000000       	mov	r9, qword ptr [r9 + 0xe8]
;;   42:	 4d89cc               	mov	r12, r9
;;   45:	 4d01d9               	add	r9, r11
;;   48:	 4439d3               	cmp	ebx, r10d
;;   4b:	 4d0f43cc             	cmovae	r9, r12
;;   4f:	 4d8b01               	mov	r8, qword ptr [r9]
;;   52:	 4c89c0               	mov	rax, r8
;;   55:	 4d85c0               	test	r8, r8
;;   58:	 0f8511000000         	jne	0x6f
;;   5e:	 4c89f7               	mov	rdi, r14
;;   61:	 be00000000           	mov	esi, 0
;;   66:	 89da                 	mov	edx, ebx
;;   68:	 ffd1                 	call	rcx
;;   6a:	 e904000000           	jmp	0x73
;;   6f:	 4883e0fe             	and	rax, 0xfffffffffffffffe
;;   73:	 4885c0               	test	rax, rax
;;   76:	 0f8420000000         	je	0x9c
;;   7c:	 4d8b5e40             	mov	r11, qword ptr [r14 + 0x40]
;;   80:	 418b0b               	mov	ecx, dword ptr [r11]
;;   83:	 8b5018               	mov	edx, dword ptr [rax + 0x18]
;;   86:	 39d1                 	cmp	ecx, edx
;;   88:	 0f8510000000         	jne	0x9e
;;   8e:	 488b4810             	mov	rcx, qword ptr [rax + 0x10]
;;   92:	 ffd1                 	call	rcx
;;   94:	 4883c410             	add	rsp, 0x10
;;   98:	 5d                   	pop	rbp
;;   99:	 c3                   	ret	
;;   9a:	 0f0b                 	ud2	
;;   9c:	 0f0b                 	ud2	
;;   9e:	 0f0b                 	ud2	
