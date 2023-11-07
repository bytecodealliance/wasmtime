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
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 c744240800000000     	mov	dword ptr [rsp + 8], 0
;;   14:	 4531db               	xor	r11d, r11d
;;   17:	 4c893424             	mov	qword ptr [rsp], r14
;;   1b:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   1f:	 b811000000           	mov	eax, 0x11
;;   24:	 85c9                 	test	ecx, ecx
;;   26:	 0f8509000000         	jne	0x35
;;   2c:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   30:	 b8ffffffff           	mov	eax, 0xffffffff
;;   35:	 4883c410             	add	rsp, 0x10
;;   39:	 5d                   	pop	rbp
;;   3a:	 c3                   	ret	
