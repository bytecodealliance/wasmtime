" Vim syntax file
" Language:     Cretonne
" Maintainer:   Jakob Stoklund Olesen <stoklund@2pi.dk
" Last Change:  Sep 22, 2016

if version < 600
  syntax clear
elseif exists("b:current_syntax")
  finish
endif

syn keyword ctonHeader test isa set
syn keyword ctonDecl function stack_slot jump_table
syn keyword ctonFilecheck check sameln nextln unordered not regex contained

syn match ctonType  /\<[bif]\d\+\(x\d\+\)\?\>/
syn match ctonEntity /\<\(v\|vx\|ss\|jt\|\)\d\+\>/
syn match ctonLabel /\<ebb\d+\>/

syn match ctonNumber /-\?\<\d\+\>/
syn match ctonNumber /-\?\<0x\x\+\(\.\x*\)\(p[+-]\?\d\+\)\?\>/

syn region ctonCommentLine start=";" end="$" contains=ctonFilecheck

hi def link ctonHeader        Keyword
hi def link ctonDecl          Keyword
hi def link ctonType          Type
hi def link ctonEntity        Identifier
hi def link ctonLabel         Label
hi def link ctonNumber        Number
hi def link ctonCommentLine   Comment
hi def link ctonFilecheck     SpecialComment

let b:current_syntax = "cton"
