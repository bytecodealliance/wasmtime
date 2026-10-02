use super::*;
use std::io::{self, ErrorKind};

// Exercise the actual conversion to each guest's error code, including the
// default hooks and the DNS special cases preceding them.
fn assert_codes(error: Error, expected: Error) {
    let mut ctx = WasiHttpCtx::new();
    let mut table = ResourceTable::new();
    let mut view = WasiHttpCtxView {
        hooks: default_hooks(),
        ctx: &mut ctx,
        table: &mut table,
    };
    assert_eq!(
        format!("{:?}", Error::from(view.error_to_p3(&error))),
        format!("{expected:?}"),
    );
    assert_eq!(
        format!("{:?}", Error::from(view.error_to_p2(error))),
        format!("{expected:?}"),
    );
}

#[test]
fn connect_error_classification() {
    for (kind, expected) in [
        (ErrorKind::ConnectionRefused, Error::ConnectionRefused),
        (ErrorKind::HostUnreachable, Error::DestinationUnavailable),
        (ErrorKind::NetworkUnreachable, Error::DestinationUnavailable),
        (ErrorKind::NetworkDown, Error::DestinationUnavailable),
        (ErrorKind::TimedOut, Error::ConnectionTimeout),
        (ErrorKind::ConnectionReset, Error::ConnectionTerminated),
        (ErrorKind::ConnectionAborted, Error::ConnectionTerminated),
        (
            ErrorKind::PermissionDenied,
            Error::InternalError(Some("connect denied".to_string())),
        ),
        (
            ErrorKind::Other,
            Error::InternalError(Some("connect denied".to_string())),
        ),
    ] {
        assert_codes(
            Error::Connect(io::Error::new(kind, "connect denied")),
            expected,
        );
    }
}

#[test]
fn unsupported_address_family() {
    assert_codes(
        Error::Connect(rustix::io::Errno::AFNOSUPPORT.into()),
        Error::DestinationUnavailable,
    );
}

#[test]
fn dns_errors_keep_their_classification() {
    for error in [
        io::Error::from(ErrorKind::AddrNotAvailable),
        io::Error::new(
            ErrorKind::Other,
            "failed to lookup address information: test",
        ),
    ] {
        assert_codes(
            Error::Connect(error),
            Error::DnsError {
                rcode: Some("address not available".to_string()),
                info_code: None,
            },
        );
    }
}

#[test]
fn tls_certificate_error() {
    assert_codes(
        Error::Tls(io::Error::new(
            ErrorKind::InvalidData,
            rustls::Error::InvalidCertificate(rustls::CertificateError::UnknownIssuer),
        )),
        Error::TlsCertificateError,
    );
}

#[test]
fn tls_alert_error() {
    // Preserve unknown wire IDs as well as named alerts.
    for (alert, message) in [
        (
            rustls::AlertDescription::HandshakeFailure,
            "HandshakeFailure",
        ),
        (
            rustls::AlertDescription::Unknown(255),
            "AlertDescription(0xff)",
        ),
    ] {
        assert_codes(
            Error::Tls(io::Error::new(
                ErrorKind::InvalidData,
                rustls::Error::AlertReceived(alert),
            )),
            Error::TlsAlertReceived {
                alert_id: Some(u8::from(alert)),
                alert_message: Some(message.to_string()),
            },
        );
    }
}

#[test]
fn tls_transport_and_protocol_errors() {
    for kind in [
        ErrorKind::ConnectionReset,
        ErrorKind::ConnectionAborted,
        ErrorKind::BrokenPipe,
        ErrorKind::UnexpectedEof,
    ] {
        assert_codes(Error::Tls(kind.into()), Error::ConnectionTerminated);
    }
    assert_codes(
        Error::Tls(io::Error::new(
            ErrorKind::InvalidData,
            rustls::Error::General("bad handshake".to_string()),
        )),
        Error::TlsProtocolError,
    );
    assert_codes(Error::Tls(ErrorKind::Other.into()), Error::TlsProtocolError);
}

#[test]
fn custom_error_hooks_still_take_precedence() {
    struct Hooks;
    impl WasiHttpHooks for Hooks {
        fn p2_error_from_connect(&mut self, _: &io::Error) -> p2::ErrorCode {
            p2::ErrorCode::ConfigurationError
        }
        fn p3_error_from_connect(&mut self, _: &io::Error) -> p3::ErrorCode {
            p3::ErrorCode::ConfigurationError
        }
        fn p2_error_from_tls(&mut self, _: &io::Error) -> p2::ErrorCode {
            p2::ErrorCode::ConfigurationError
        }
        fn p3_error_from_tls(&mut self, _: &io::Error) -> p3::ErrorCode {
            p3::ErrorCode::ConfigurationError
        }
    }
    let mut ctx = WasiHttpCtx::new();
    let mut table = ResourceTable::new();
    let mut view = WasiHttpCtxView {
        hooks: &mut Hooks,
        ctx: &mut ctx,
        table: &mut table,
    };
    for error in [
        Error::Connect(ErrorKind::HostUnreachable.into()),
        Error::Tls(io::Error::new(
            ErrorKind::InvalidData,
            rustls::Error::InvalidCertificate(rustls::CertificateError::UnknownIssuer),
        )),
    ] {
        assert!(matches!(
            view.error_to_p3(&error),
            p3::ErrorCode::ConfigurationError
        ));
        assert!(matches!(
            view.error_to_p2(error),
            p2::ErrorCode::ConfigurationError
        ));
    }
}

#[tokio::test]
async fn real_tls_handshake_errors() {
    use http_body_util::{BodyExt, Empty};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    for (reply, expected) in [
        // FIN after receiving a ClientHello.
        (&b""[..], Error::ConnectionTerminated),
        // A fatal handshake_failure TLS alert with a real wire ID.
        (
            &b"\x15\x03\x03\x00\x02\x02\x28"[..],
            Error::TlsAlertReceived {
                alert_id: Some(40),
                alert_message: Some("HandshakeFailure".to_string()),
            },
        ),
        (&b"HTTP/1.1 200 OK\r\n\r\n"[..], Error::TlsProtocolError),
    ] {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let server = async {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut hello = [0; 4096];
            assert!(stream.read(&mut hello).await.unwrap() > 0);
            stream.write_all(reply).await.unwrap();
            stream.shutdown().await.unwrap();
        };
        let client = async {
            let request = http::Request::builder()
                .uri(format!("https://{addr}/"))
                .body(Empty::<Bytes>::new().map_err(Error::from))
                .unwrap();
            match crate::default_send_request(request, None).await {
                Err(error @ Error::Tls(_)) => assert_codes(error, expected),
                Err(error) => panic!("expected a TLS handshake error: {error:?}"),
                Ok(_) => panic!("unexpected successful TLS handshake"),
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            tokio::join!(server, client);
        })
        .await
        .unwrap();
    }
}
