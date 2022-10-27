use std::path::PathBuf;

use veri_annotation::parser_wrapper::{parse_annotations, parse_annotations_str};
use veri_ir::isle_annotations::isle_annotation_for_term;

#[test]
fn test_parser_single_file() {
    let annotation_env = parse_annotations(&vec![PathBuf::from("examples/simple.isle")]);
    assert_eq!(annotation_env.annotation_map.len(), 3);
    for (term, annotation) in annotation_env.annotation_map {
        let expected = isle_annotation_for_term(&term).unwrap();
        assert_eq!(expected, annotation);
    }
}

#[test]
fn test_parser_multi_file() {
    let annotation_env = parse_annotations(&vec![
        PathBuf::from("examples/simple.isle"),
        PathBuf::from("examples/simple2.isle"),
    ]);
    assert_eq!(annotation_env.annotation_map.len(), 4);
    for (term, annotation) in annotation_env.annotation_map {
        let expected = isle_annotation_for_term(&term).unwrap();
        assert_eq!(expected, annotation);
    }
}

#[test]
fn test_parser_str() {
    let code = "
        ;;@ (spec (sig (args arg) (ret))
        ;;@     (assertions (= (arg) (ret)), (<= (arg) (64i128: isleType))))
        (decl fits_in_64 (Type) Type)
        (extern extractor fits_in_64 fits_in_64)
        
        (decl fits_in_32 (Type) Type)
        (extern extractor fits_in_32 fits_in_32)
        
        ;;@ (spec (sig (args a, b) (r))
        ;;@     (assertions (= (+ (a) (b)) (r))))
        (decl iadd (Value Value) Inst)
        (extern extractor iadd iadd)
    ";
    let annotation_env = parse_annotations_str(code);

    assert_eq!(annotation_env.annotation_map.len(), 2);
    for (term, annotation) in annotation_env.annotation_map {
        let expected = isle_annotation_for_term(&term).unwrap();
        assert_eq!(expected, annotation);
    }
}

#[test]
#[should_panic]
fn test_parser_no_decl() {
    let code = "
        ;;@ (spec (sig (args arg) (ret))
        ;;@     (assertions (= (arg) (ret)), (<= (arg) (64i128: isleType))))
        (extern extractor fits_in_64 fits_in_64)
    ";
    parse_annotations_str(code);
}

#[test]
#[should_panic]
fn test_parser_dup_term_same_file() {
    let code = "
        ;;@ (spec (sig (args arg) (ret))
        ;;@     (assertions (= (arg) (ret)), (<= (arg) (64i128: isleType))))
        (decl fits_in_64 (Type) Type)
        
        ;;@ (spec (sig (args arg) (ret))
        ;;@     (assertions (= (arg) (ret)), (<= (arg) (32: isleType))))
        (decl fits_in_32 (Type) Type)
        (extern extractor fits_in_32 fits_in_32)
        
        ;;@ (spec (sig (args arg) (ret))
        ;;@     (assertions (= (arg) (ret)), (<= (arg) (16i128: isleType))))
        (decl fits_in_64 (Type) Type)
    ";
    parse_annotations_str(code);
}

#[test]
#[should_panic]
fn test_parser_dup_term_diff_files() {
    parse_annotations(&vec![
        PathBuf::from("examples/simple.isle"),
        PathBuf::from("examples/simple.isle"),
    ]);
}
