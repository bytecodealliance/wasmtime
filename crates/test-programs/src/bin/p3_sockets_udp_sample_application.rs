use futures::join;
use test_programs::p3::wasi::sockets::types::{
    IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress, UdpSocket,
};

struct Component;

test_programs::p3::export!(Component);

async fn test_udp_sample_application(family: IpAddressFamily, bind_address: IpSocketAddress) {
    let unspecified_addr = IpSocketAddress::new(IpAddress::new_unspecified(family), 0);

    let first_message = &[];
    let second_message = b"Hello, world!";
    let third_message = b"Greetings, planet!";

    let server = UdpSocket::new(family);

    server.bind(bind_address).unwrap();
    let addr = server.local_address().unwrap();

    let client = UdpSocket::new(family);
    client.bind(unspecified_addr).unwrap();
    client.connect(addr).unwrap();
    let client_addr = client.local_address().unwrap();
    join!(
        async {
            client.send(first_message.to_vec(), None).await.unwrap();
            client
                .send(second_message.to_vec(), Some(addr))
                .await
                .unwrap();
        },
        async {
            // Check that we've received our sent messages.
            let (buf, addr) = server.receive().await.unwrap();
            assert_eq!(buf, first_message);
            assert_eq!(addr, client_addr);

            let (buf, addr) = server.receive().await.unwrap();
            assert_eq!(buf, second_message);
            assert_eq!(addr, client_addr);
        }
    );
    join!(
        async {
            // Another client
            let client = UdpSocket::new(family);
            client.bind(unspecified_addr).unwrap();
            client
                .send(third_message.to_vec(), Some(addr))
                .await
                .unwrap();
        },
        async {
            // Check that we sent and received our message!
            let (buf, _) = server.receive().await.unwrap();
            assert_eq!(buf, third_message);
        },
    );
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_udp_sample_application(
            IpAddressFamily::Ipv4,
            IpSocketAddress::Ipv4(Ipv4SocketAddress {
                port: 0,                 // use any free port
                address: (127, 0, 0, 1), // localhost
            }),
        )
        .await;
        test_udp_sample_application(
            IpAddressFamily::Ipv6,
            IpSocketAddress::Ipv6(Ipv6SocketAddress {
                port: 0,                           // use any free port
                address: (0, 0, 0, 0, 0, 0, 0, 1), // localhost
                flow_info: 0,
                scope_id: 0,
            }),
        )
        .await;
        Ok(())
    }
}

fn main() {}
