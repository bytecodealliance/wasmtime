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
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 b800000000           	mov	eax, 0
;;   1a:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1e:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   23:	 4153                 	push	r11
;;   25:	 b801000000           	mov	eax, 1
;;   2a:	 59                   	pop	rcx
;;   2b:	 01c1                 	add	ecx, eax
;;   2d:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   31:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   36:	 4153                 	push	r11
;;   38:	 b802000000           	mov	eax, 2
;;   3d:	 59                   	pop	rcx
;;   3e:	 01c1                 	add	ecx, eax
;;   40:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   44:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   49:	 4153                 	push	r11
;;   4b:	 b804000000           	mov	eax, 4
;;   50:	 59                   	pop	rcx
;;   51:	 01c1                 	add	ecx, eax
;;   53:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   57:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   5c:	 4153                 	push	r11
;;   5e:	 b808000000           	mov	eax, 8
;;   63:	 59                   	pop	rcx
;;   64:	 01c1                 	add	ecx, eax
;;   66:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   6a:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   6f:	 4153                 	push	r11
;;   71:	 b810000000           	mov	eax, 0x10
;;   76:	 59                   	pop	rcx
;;   77:	 01c1                 	add	ecx, eax
;;   79:	 894c240c             	mov	dword ptr [rsp + 0xc], ecx
;;   7d:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   81:	 4883c410             	add	rsp, 0x10
;;   85:	 5d                   	pop	rbp
;;   86:	 c3                   	ret	
