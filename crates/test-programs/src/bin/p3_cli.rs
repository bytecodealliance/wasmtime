use test_programs::p3::wasi::cli::{
    environment, stderr, stdin, stdout, terminal_stderr, terminal_stdin, terminal_stdout,
};
use test_programs::p3::wit_stream;
use wit_bindgen::StreamResult;

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        assert_eq!(environment::get_arguments(), ["p3_cli.component", "."]);
        assert_ne!(environment::get_environment(), []);
        assert_eq!(environment::get_initial_cwd(), None);

        assert!(terminal_stdin::get_terminal_stdin().is_none());
        assert!(terminal_stdout::get_terminal_stdout().is_none());
        assert!(terminal_stderr::get_terminal_stderr().is_none());

        let (mut stdin, result) = stdin::read_via_stream();
        assert!(stdin.next().await.is_none());

        let (mut stdout_tx, stdout_rx) = wit_stream::new();
        futures::join!(
            async {
                stdout::write_via_stream(stdout_rx).await.unwrap();
            },
            async {
                let (res, buf) = stdout_tx.write(b"hello stdout\n".into()).await;
                assert_eq!(res, StreamResult::Complete(13));
                assert_eq!(buf.into_vec(), []);
                drop(stdout_tx);
            }
        );

        let (mut stderr_tx, stderr_rx) = wit_stream::new();
        futures::join!(
            async {
                stderr::write_via_stream(stderr_rx).await.unwrap();
            },
            async {
                let (res, buf) = stderr_tx.write(b"hello stderr\n".into()).await;
                assert_eq!(res, StreamResult::Complete(13));
                assert_eq!(buf.into_vec(), []);
                drop(stderr_tx);
            }
        );

        drop(stdin);
        result.await.unwrap();

        Ok(())
    }
}

fn main() {}
