fn main() {
    let fields = test_programs::wasi::http::types::Fields::new();

    match std::env::args().nth(1).as_deref() {
        Some("append") => {
            for i in 0.. {
                if fields.append(&format!("a{i}"), &b"a".to_vec()).is_err() {
                    break;
                }
            }
        }
        Some("append-empty") => {
            for i in 0.. {
                if fields.append(&format!("a{i}"), &Vec::new()).is_err() {
                    break;
                }
            }
        }
        Some("append-same") => loop {
            if fields.append(&"a".to_string(), &b"b".to_vec()).is_err() {
                break;
            }
        },
        Some("append-same-empty") => loop {
            if fields.append(&"a".to_string(), &Vec::new()).is_err() {
                break;
            }
        },
        other => panic!("unknown test {other:?}"),
    }

    unreachable!();
}
