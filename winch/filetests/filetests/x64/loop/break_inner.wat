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
;;   26:	 b801000000           	mov	eax, 1
;;   2b:	 59                   	pop	rcx
;;   2c:	 01c1                 	add	ecx, eax
;;   2e:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   32:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   37:	 4153                 	push	r11
;;   39:	 b802000000           	mov	eax, 2
;;   3e:	 59                   	pop	rcx
;;   3f:	 01c1                 	add	ecx, eax
;;   41:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   45:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   4a:	 4153                 	push	r11
;;   4c:	 b804000000           	mov	eax, 4
;;   51:	 59                   	pop	rcx
;;   52:	 01c1                 	add	ecx, eax
;;   54:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   58:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   5d:	 4153                 	push	r11
;;   5f:	 b808000000           	mov	eax, 8
;;   64:	 59                   	pop	rcx
;;   65:	 01c1                 	add	ecx, eax
;;   67:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   6b:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   70:	 4153                 	push	r11
;;   72:	 b810000000           	mov	eax, 0x10
;;   77:	 59                   	pop	rcx
;;   78:	 01c1                 	add	ecx, eax
;;   7a:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   7e:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   82:	 4883c410             	add	rsp, 0x10
;;   86:	 5d                   	pop	rbp
;;   87:	 c3                   	ret	
