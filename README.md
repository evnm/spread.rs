1;2c# spread-rs

A Rust client library for the Spread toolkit.

Spread is a toolkit that provides a high performance messaging service
that is resilient to faults across external or internal networks. More
info at [spread.org](http://www.spread.org/).

## Project status

This library is a simplified implementation of the Spread client
protocol. It supports the six basic calls of the Spread API:

- connect to a daemon
- disconnect from a daemon
- join a group
- leave a group
- multicast a message
- receive a message

No further functionality is currently implemented (e.g. no message
types beyond simple reliable multicast, quality of service, connection
prioritization, non-null authentication).

## Build usage

`spread-rs` has a single external library dependency upon
[rust-encoding](https://github.com/lifthrasiir/rust-encoding).

To build:

    $ cargo build

To test:

    $ cargo test

To generate documentation:

    $ cargo doc

## API usage

Connect to a Spread daemon running locally on port 4803:

    extern crate spread;

    use std::io::net::ip::SocketAddr;

    let socket_addr =
        from_str::<SocketAddr>("127.0.0.1:4803").expect("malformed address");
    let client = spread::connect(socket_addr, "test_user", false)
        .ok().expect("failed to create client");

Join a group and multicast a message:

    client.join("foo_group".as_slice());
    client.multicast(["foo_group"], "hello".as_bytes());

Block on receipt of a message, print the contents, and then leave and
disconnect:

    let msg = client.receive().ok().expect("receive failed");
    println!("sender: {}", msg.sender);
    println!("groups: {}", msg.groups);
    println!("data: {}", msg.data);

    client.leave("foo_group".as_slice());
    client.disconnect();
