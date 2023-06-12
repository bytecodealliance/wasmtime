extern crate proc_macro;

use proc_macro::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};

/// Expand a `str` literal into a byte array.
#[proc_macro]
pub fn str(input: TokenStream) -> TokenStream {
    let rv = convert_str(input);

    vec![TokenTree::Group(Group::new(
        Delimiter::Bracket,
        rv.into_iter().collect(),
    ))]
    .into_iter()
    .collect()
}

/// The same as `str` but appends a `'\n'`.
#[proc_macro]
pub fn str_nl(input: TokenStream) -> TokenStream {
    let mut rv = convert_str(input);

    rv.push(TokenTree::Literal(Literal::u8_suffixed(b'\n')));

    vec![TokenTree::Group(Group::new(
        Delimiter::Bracket,
        rv.into_iter().collect(),
    ))]
    .into_iter()
    .collect()
}

fn convert_str(input: TokenStream) -> Vec<TokenTree> {
    let mut it = input.into_iter();

    let mut tokens = Vec::new();
    match it.next() {
        Some(TokenTree::Literal(l)) => {
            for b in to_string(l).into_bytes() {
                tokens.push(TokenTree::Literal(Literal::u8_suffixed(b)));
                tokens.push(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
            }
        }
        _ => panic!(),
    }

    assert!(it.next().is_none());
    tokens
}

fn to_string(lit: Literal) -> String {
    let formatted = lit.to_string();

    let mut it = formatted.chars();
    assert_eq!(it.next(), Some('"'));

    let mut rv = String::new();
    loop {
        match it.next() {
            Some('"') => match it.next() {
                Some(_) => panic!(),
                None => break,
            },
            Some('\\') => match it.next() {
                Some('x') => {
                    let hi = it.next().unwrap().to_digit(16).unwrap();
                    let lo = it.next().unwrap().to_digit(16).unwrap();
                    let v = (hi << 16) | lo;
                    rv.push(v as u8 as char);
                }
                Some('u') => {
                    assert_eq!(it.next(), Some('{'));
                    let mut c = it.next().unwrap();
                    let mut ch = 0;
                    while let Some(v) = c.to_digit(16) {
                        ch *= 16;
                        ch |= v;
                        c = it.next().unwrap();
                    }
                    assert_eq!(c, '}');
                    rv.push(::std::char::from_u32(ch).unwrap());
                }
                Some('0') => rv.push('\0'),
                Some('\\') => rv.push('\\'),
                Some('\"') => rv.push('\"'),
                Some('r') => rv.push('\r'),
                Some('n') => rv.push('\n'),
                Some('t') => rv.push('\t'),
                Some(_) => panic!(),
                None => panic!(),
            },
            Some(c) => rv.push(c),
            None => panic!(),
        }
    }

    rv
}
