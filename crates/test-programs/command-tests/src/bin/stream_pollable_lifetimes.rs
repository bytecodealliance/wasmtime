use command_tests::wasi::cli_base::environment;
use command_tests::wasi::cli_base::stdin;
use command_tests::wasi::io::streams;
use command_tests::wasi::poll::poll;

fn main() {
    let args = environment::get_arguments();

    if args == &["correct"] {
        let stdin: streams::InputStream = stdin::get_stdin();
        let stdin_pollable = streams::subscribe_to_input_stream(stdin);
        let ready = poll::poll_oneoff(&[stdin_pollable]);
        assert_eq!(ready, &[true]);
        poll::drop_pollable(stdin_pollable);
        streams::drop_input_stream(stdin);
    } else if args == &["trap"] {
        let stdin: streams::InputStream = stdin::get_stdin();
        let stdin_pollable = streams::subscribe_to_input_stream(stdin);
        let ready = poll::poll_oneoff(&[stdin_pollable]);
        assert_eq!(ready, &[true]);
        streams::drop_input_stream(stdin);
        unreachable!(
            "execution should have trapped in line above when stream dropped before pollable"
        );
    } else {
        panic!("bad value for args: expected `[\"correct\"]` or `[\"trap\"]`, got {args:?}")
    }
}
