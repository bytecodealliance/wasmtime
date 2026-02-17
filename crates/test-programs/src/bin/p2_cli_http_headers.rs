fn main() {
    let fields = wasip2::http::types::Fields::new();

    match std::env::args().nth(1).as_deref() {
        Some("append") => {
            for i in 0.. {
                if fields.append(&format!("a{i}"), b"a").is_err() {
                    break;
                }
            }
        }
        Some("append-empty") => {
            for i in 0.. {
                if fields.append(&format!("a{i}"), b"").is_err() {
                    break;
                }
            }
        }
        Some("append-same") => loop {
            if fields.append("a", b"b").is_err() {
                break;
            }
        },
        Some("append-same-empty") => loop {
            if fields.append("a", b"").is_err() {
                break;
            }
        },
        other => panic!("unknown test {other:?}"),
    }

    unreachable!();
}
