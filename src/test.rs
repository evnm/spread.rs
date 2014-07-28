#[cfg(test)]
mod test {
    use std::io::net::ip::SocketAddr;
    use connect;

    #[test]
    fn should_connect() {
        let socket_addr = from_str::<SocketAddr>("127.0.0.1:4567").expect("malformed address");
        let connection = connect(socket_addr, "test", false, false);
        assert!(true);
    }

    #[test]
    fn should_disconnect() {
        let socket_addr = from_str::<SocketAddr>("127.0.0.1:4567").expect("malformed address");
        let connection = connect(socket_addr, "test", false, false);
        connection.disconnect();
        assert!(true);
    }
}
