use test_programs::p3::wasi as wasip3;

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("p2-append") => {
            let fields = wasip2::http::types::Fields::new();
            for i in 0.. {
                if fields.append(&format!("a{i}"), b"a").is_err() {
                    break;
                }
            }
        }
        Some("p2-append-empty") => {
            let fields = wasip2::http::types::Fields::new();
            for i in 0.. {
                if fields.append(&format!("a{i}"), b"").is_err() {
                    break;
                }
            }
        }
        Some("p2-append-same") => {
            let fields = wasip2::http::types::Fields::new();
            loop {
                if fields.append("a", b"b").is_err() {
                    break;
                }
            }
        }
        Some("p2-append-same-empty") => {
            let fields = wasip2::http::types::Fields::new();
            loop {
                if fields.append("a", b"").is_err() {
                    break;
                }
            }
        }
        Some("p3-append") => {
            let fields = wasip3::http::types::Fields::new();
            for i in 0.. {
                if fields.append(&format!("a{i}"), b"a").is_err() {
                    break;
                }
            }
        }
        Some("p3-append-empty") => {
            let fields = wasip3::http::types::Fields::new();
            for i in 0.. {
                if fields.append(&format!("a{i}"), b"").is_err() {
                    break;
                }
            }
        }
        Some("p3-append-same") => {
            let fields = wasip3::http::types::Fields::new();
            loop {
                if fields.append("a", b"b").is_err() {
                    break;
                }
            }
        }
        Some("p3-append-same-empty") => {
            let fields = wasip3::http::types::Fields::new();
            loop {
                if fields.append("a", b"").is_err() {
                    break;
                }
            }
        }
        other => panic!("unknown test {other:?}"),
    }

    unreachable!();
}
