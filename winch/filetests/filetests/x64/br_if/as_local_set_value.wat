;;! target = "x86_64"
(module
  (func (export "as-local-set-value") (param i32) (result i32)
    (local i32)
    (block (result i32)
      (local.set 0 (br_if 0 (i32.const 17) (local.get 0)))
      (i32.const -1)
    )
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8733000000         	ja	0x4b
;;   18:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 c744240800000000     	mov	dword ptr [rsp + 8], 0
;;      	 4531db               	xor	r11d, r11d
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;      	 b811000000           	mov	eax, 0x11
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8509000000         	jne	0x45
;;   3c:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;      	 b8ffffffff           	mov	eax, 0xffffffff
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   4b:	 0f0b                 	ud2	
