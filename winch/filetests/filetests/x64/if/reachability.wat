;;! target = "x86_64"
(module
  (func (;0;) (param i32) (result i32)
    local.get 0
    local.get 0
    if (result i32)
      i32.const 1
        return
      else
        i32.const 2
      end
      i32.sub
  )
  (export "main" (func 0))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c31c000000       	add	r11, 0x1c
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f874f000000         	ja	0x6d
;;   1e:	 4883ec18             	sub	rsp, 0x18
;;      	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;      	 4889742408           	mov	qword ptr [rsp + 8], rsi
;;      	 89542404             	mov	dword ptr [rsp + 4], edx
;;      	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;      	 448b5c2404           	mov	r11d, dword ptr [rsp + 4]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 85c0                 	test	eax, eax
;;      	 0f840e000000         	je	0x57
;;   49:	 b801000000           	mov	eax, 1
;;      	 4883c404             	add	rsp, 4
;;      	 e910000000           	jmp	0x67
;;   57:	 b802000000           	mov	eax, 2
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 29c1                 	sub	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   6d:	 0f0b                 	ud2	
