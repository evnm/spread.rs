#[cfg(test)]
mod test {
    use std::io::net::ip::SocketAddr;
    use {connect, encode_connect_message, SpreadClient};

    #[test]
    fn should_encode_connect_message_with_sufficiently_short_private_name() {
        let result = encode_connect_message("test", true, true);
        assert_eq!(result, vec!(4, 4, 0, 17, 4, 116, 101, 115, 116));
    }

    #[test]
    fn should_convert_ints_to_byte_vectors() {
        assert_eq!(SpreadClient::int_to_bytes(0), vec!(0 as u8, 0, 0, 0));
        assert_eq!(SpreadClient::int_to_bytes(0x00040000), vec!(0 as u8, 4, 0, 0));
        assert_eq!(
            SpreadClient::int_to_bytes(0xffffffff),
            vec!(255 as u8, 255, 255, 255)
        );
    }

    #[test]
    fn should_encode_service_message() {
        match SpreadClient::encode_message(
            0x00010000, "de", ["ad"], "beef".as_bytes()
        ) {
            Ok(result) => {
                assert_eq!(
                    result,
                    vec!(0, 1, 0, 0, 78, 85, 76, 76, 0, 0, 0, 1, 0, 0, 0,
                         0, 0, 0, 0, 4, 78, 85, 76, 76, 98, 101, 101, 102)
                );
            },
            Err(error) => {
                println!("{}", error);
                fail!(error);
            }
        }
    }

    #[test]
    fn should_connect_and_disconnect() {
        let socket_addr =
            from_str::<SocketAddr>("127.0.0.1:4803").expect("malformed address");
        let result = connect(socket_addr, "test", false, false);
        match result {
            Ok(mut client) => {
                client.disconnect();
                assert!(true);
            },
            Err(error) => {
                println!("{}", error);
                fail!(error);
            }
        }
    }
}
