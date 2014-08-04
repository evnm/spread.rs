#[cfg(test)]
mod test {
    use {connect, encode_connect_message, SpreadClient};
    use encoding::{Encoding, EncodeStrict};
    use encoding::all::ISO_8859_1;
    use std::io::net::ip::SocketAddr;
    use util::{int_to_bytes, bytes_to_int};

    #[test]
    fn should_encode_connect_message_with_sufficiently_short_private_name() {
        match encode_connect_message("test", true) {
            Ok(result) => assert_eq!(result, vec!(4, 4, 0, 16, 4, 116, 101, 115, 116)),
            Err(error) => fail!(error)
        }
    }

    #[test]
    fn should_convert_int_to_byte_vector() {
        assert_eq!(int_to_bytes(0), vec!(0 as u8, 0, 0, 0));
        assert_eq!(int_to_bytes(0x00040000), vec!(0 as u8, 4, 0, 0));
        assert_eq!(int_to_bytes(0xffffffff), vec!(255 as u8, 255, 255, 255));
    }

    #[test]
    fn should_convert_byte_vector_to_int() {
        assert_eq!(bytes_to_int([160 as u8, 0, 0, 128]), 2684354688);
    }

    #[test]
    fn should_encode_service_message() {
        match SpreadClient::encode_message(0x00010000, "de", ["ad"], "beef".as_bytes()) {
            Ok(result) => assert_eq!(
                result,
                vec!(0, 1, 0, 0, 100, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                     0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 4, 97, 100, 0, 0,
                     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 98, 101, 101, 102)
            ),
            Err(error) => fail!(error)
        }
    }

    // Integration tests -- requires a locally-running Spread daemon, so these
    // are left un-`#[test]`-ed.

    //#[test]
    fn should_connect_and_disconnect() {
        let socket_addr =
            from_str::<SocketAddr>("127.0.0.1:4803").expect("malformed address");
        let result = connect(socket_addr, "test_user", false);
        match result {
            Ok(mut client) => {
                let msg = ISO_8859_1.encode("hello".as_slice(), EncodeStrict)
                    .ok().expect("message encoding failed");
                assert!(client.join("foo".as_slice()).is_ok());
                assert!(client.multicast(["foo"], msg.as_slice()).is_ok());
                assert!(client.leave("foo".as_slice()).is_ok());
                assert!(client.disconnect().is_ok());
            },
            Err(error) => fail!(error)
        }
    }

    //#[test]
    fn should_receive() {
        let socket_addr =
            from_str::<SocketAddr>("127.0.0.1:4803").expect("malformed address");
        let result = connect(socket_addr, "test_user", true);
        match result {
            Ok(mut client) => {
                assert!(client.join("foo".as_slice()).is_ok());
                let msg = client.receive().ok().expect("receive failed");
                println!("sender: {}", msg.sender);
                println!("groups: {}", msg.groups);
                println!("data: {}", msg.data);
                assert!(client.disconnect().is_ok());
                // fail the test so that stdout is printed.
                assert!(false);
            },
            Err(error) => fail!(error)
        }
    }
}
