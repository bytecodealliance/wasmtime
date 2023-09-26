use command_tests::wasi::cli::environment;
use command_tests::wasi::cli::stdin;
use command_tests::wasi::io::poll;
use command_tests::wasi::io::streams;

fn main() {
    let args = environment::get_arguments();

    if args == &["correct"] {
        let stdin: streams::InputStream = stdin::get_stdin();
        let stdin_pollable = stdin.subscribe();
        let ready = poll::poll_list(&[&stdin_pollable]);
        assert_eq!(ready, &[0]);
        drop(stdin_pollable);
        drop(stdin);
    } else if args == &["trap"] {
        let stdin: streams::InputStream = stdin::get_stdin();
        let stdin_pollable = stdin.subscribe();
        let ready = poll::poll_list(&[&stdin_pollable]);
        assert_eq!(ready, &[0]);
        drop(stdin);
        unreachable!(
            "execution should have trapped in line above when stream dropped before pollable"
        );
    } else {
        panic!("bad value for args: expected `[\"correct\"]` or `[\"trap\"]`, got {args:?}")
    }
}
