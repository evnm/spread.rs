#[cfg(test)]
mod test {
    use std::io::net::ip::SocketAddr;
    use encode_connect_message;
    use connect;

    #[test]
    fn should_encode_connect_message_with_sufficiently_short_private_name() {
        let result = encode_connect_message("test", true, true);
        assert_eq!(result, vec!(4, 4, 0, 17, 4, 116, 101, 115, 116));
    }

    #[test]
    fn should_connect() {
        let socket_addr = from_str::<SocketAddr>("127.0.0.1:4803").expect("malformed address");
        let result = connect(socket_addr, "test", false, false);
        match result {
            Ok(_) => assert!(true),
            Err(error) => {
                println!("{}", error);
                fail!(error);
            }
        }
    }
}
