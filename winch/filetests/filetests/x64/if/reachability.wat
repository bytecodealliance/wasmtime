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
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8745000000         	ja	0x5d
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 85c0                 	test	eax, eax
;;      	 0f840e000000         	je	0x47
;;   39:	 b801000000           	mov	eax, 1
;;      	 4883c404             	add	rsp, 4
;;      	 e910000000           	jmp	0x57
;;   47:	 b802000000           	mov	eax, 2
;;      	 8b0c24               	mov	ecx, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 29c1                 	sub	ecx, eax
;;      	 89c8                 	mov	eax, ecx
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5d:	 0f0b                 	ud2	
