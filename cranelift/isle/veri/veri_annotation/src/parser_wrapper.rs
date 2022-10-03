use std::collections::HashMap;
use std::path::PathBuf;

use crate::parser::TermAnnotationParser;
use veri_ir::annotation_ir::TermAnnotation;

fn get_term(isle: &str) -> &str {
    assert!(isle.contains("decl"));
    let tokens: Vec<&str> = isle.split(' ').collect();
    let first_token = tokens[1];
    // Stopgap for now
    if first_token != "pure" {
        first_token
    } else {
        tokens[2]
    }
}

#[derive(Clone, Debug)]
pub struct AnnotationEnv {
    pub annotation_map: HashMap<String, TermAnnotation>,
}

impl AnnotationEnv {
    pub fn new(annotation_env: HashMap<String, TermAnnotation>) -> Self {
        AnnotationEnv {
            annotation_map: annotation_env,
        }
    }

    pub fn get_annotation_for_term(&self, term: &str) -> Option<TermAnnotation> {
        if self.annotation_map.contains_key(term) {
            return Some(self.annotation_map[term].clone());
        }
        None
    }
}

// Assume every term has at most one definition.
pub fn parse_annotations(files: &Vec<PathBuf>) -> AnnotationEnv {
    let mut annotation_env = HashMap::new();

    for file in files {
        let code = std::fs::read_to_string(file).unwrap();
        let a = parse_annotations_str(&code);

        for k in a.annotation_map.keys() {
            assert!(
                !annotation_env.contains_key(k),
                "double definition for key: {:?}",
                k
            );
            annotation_env.insert(k.clone(), a.annotation_map[k].clone());
        }
    }

    AnnotationEnv {
        annotation_map: annotation_env,
    }
}

pub fn parse_annotations_str(code: &str) -> AnnotationEnv {
    let mut annotation_env = HashMap::new();
    let p = TermAnnotationParser::new();
    let mut lines = code.lines();

    while let Some(l) = lines.next() {
        let line = l.trim_start().trim_end();
        let mut cur = String::from("");

        // ignore lines that don't start with ;;@
        if line.len() < 3 || &line[..3] != ";;@" {
            continue;
        }

        // lines that begin with ;;@ are part of annotations
        let mut next_line = line;
        while next_line.len() >= 3 && &next_line[..3] == ";;@" {
            cur += &next_line[3..];
            if let Some(annotation_line) = lines.next() {
                next_line = annotation_line.trim_start().trim_end();
            }
        }

        let annotation = p.parse(&cur).unwrap();

        // parse the term associated with the annotation
        let term = get_term(next_line).to_owned();
        assert!(!annotation_env.contains_key(&term));
        annotation_env.insert(term, annotation);
    }

    AnnotationEnv {
        annotation_map: annotation_env,
    }
}
