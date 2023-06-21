;;! target = "x86_64"
(module
  (func (export "break-inner") (result i32)
    (local i32)
    (local.set 0 (i32.const 0))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (block (result i32) (br 2 (i32.const 0x1)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (loop (result i32) (br 2 (i32.const 0x2)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (block (result i32) (loop (result i32) (br 1 (i32.const 0x4))))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (i32.ctz (br 1 (i32.const 0x8)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (i32.ctz (loop (result i32) (br 2 (i32.const 0x10))))))))
    (local.get 0)
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   16:	 b800000000           	mov	eax, 0
;;   1b:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1f:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   24:	 4153                 	push	r11
;;   26:	 48c7c001000000       	mov	rax, 1
;;   2d:	 59                   	pop	rcx
;;   2e:	 01c1                 	add	ecx, eax
;;   30:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   34:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   39:	 4153                 	push	r11
;;   3b:	 48c7c002000000       	mov	rax, 2
;;   42:	 59                   	pop	rcx
;;   43:	 01c1                 	add	ecx, eax
;;   45:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   49:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   4e:	 4153                 	push	r11
;;   50:	 48c7c004000000       	mov	rax, 4
;;   57:	 59                   	pop	rcx
;;   58:	 01c1                 	add	ecx, eax
;;   5a:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   5e:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   63:	 4153                 	push	r11
;;   65:	 48c7c008000000       	mov	rax, 8
;;   6c:	 59                   	pop	rcx
;;   6d:	 01c1                 	add	ecx, eax
;;   6f:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   73:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   78:	 4153                 	push	r11
;;   7a:	 48c7c010000000       	mov	rax, 0x10
;;   81:	 59                   	pop	rcx
;;   82:	 01c1                 	add	ecx, eax
;;   84:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   88:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   8c:	 4883c410             	add	rsp, 0x10
;;   90:	 5d                   	pop	rbp
;;   91:	 c3                   	ret	
