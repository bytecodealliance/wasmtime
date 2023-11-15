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
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c893424             	mov	qword ptr [rsp], r14
;;   10:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   14:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   19:	 4883ec04             	sub	rsp, 4
;;   1d:	 44891c24             	mov	dword ptr [rsp], r11d
;;   21:	 85c0                 	test	eax, eax
;;   23:	 0f840e000000         	je	0x37
;;   29:	 b801000000           	mov	eax, 1
;;   2e:	 4883c404             	add	rsp, 4
;;   32:	 e910000000           	jmp	0x47
;;   37:	 b802000000           	mov	eax, 2
;;   3c:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   3f:	 4883c404             	add	rsp, 4
;;   43:	 29c1                 	sub	ecx, eax
;;   45:	 89c8                 	mov	eax, ecx
;;   47:	 4883c410             	add	rsp, 0x10
;;   4b:	 5d                   	pop	rbp
;;   4c:	 c3                   	ret	
