#![crate_id = "spread#0.0.1"]
#![crate_type = "lib"]
#![comment = "A Rust client library for the Spread toolkit"]
#![license = "MIT"]

#[deny(non_camel_case_types)]

use std::io::net::ip::SocketAddr;

mod test;

pub static DefaultSpreadPort: i16 = 4803;
static MaxPrivateNameLength: uint = 10;

// Error codes, as per http://www.spread.org/docs/spread_docs_4/docs/error_codes.html
pub enum SpreadError {
    AcceptSession = 1,
    IllegalSpread = -1,
    CouldNotConnection = -2,
    RejectQuota = -3,
    RejectNOName = -4,
    RejectIllegalName = -5,
    RejectNotUnique = -6,
    RejectVersion = -7,
    ConnectionClosed = -8,
    RejectAuth = -9,
    IllegalSession = -11,
    IllegalService = -12,
    IllegalMessage = -13,
    IllegalGroup = -14,
    BufferTooShort = -15,
    GroupsTooShort = -16,
    MessageTooLong = -17,
    NetErrorOnSession = -18
}

pub struct SpreadConnection {
    addr: SocketAddr,
    name: String,
    is_priority_connection: bool,
    receive_membership_messages: bool
}

pub struct SpreadGroup {
    name: String
}

pub struct SpreadMessageHeader {
    recipients: Vec<SpreadGroup>
}

pub fn connect(
    addr: SocketAddr,
    private_name: &str,
    is_priority_connection: bool,
    receive_membership_messages: bool
) -> SpreadConnection {
    // Send the initial connect message.
    let mut buf: Vec<u8> = Vec::new();

    // Set Spread version.
    // TODO: constants for elements.
    buf.push(4);
    buf.push(4);
    buf.push(0);

    // Apply masks for group membership and priority.
    let masked = match (is_priority_connection, receive_membership_messages) {
        (true, true) => 1 | 16,
        (true, false) => 1,
        (false, true) => 16,
        (false, false) => 0
    };
    buf.push(masked);

    // Truncate (if necessary) and write `private_name`.
    let truncated_private_name = match private_name {
        too_long if too_long.char_len() > MaxPrivateNameLength =>
            too_long.slice_to(MaxPrivateNameLength),
        just_fine => just_fine
    };
    buf.push(truncated_private_name.char_len() as u8);
    buf.push_all(truncated_private_name.as_bytes());

    // TODO: Send the connect message.

    SpreadConnection {
        addr: addr,
        name: String::from_str(truncated_private_name),
        is_priority_connection: is_priority_connection,
        receive_membership_messages: receive_membership_messages
    }
}

impl SpreadConnection {
    fn disconnect(&self) {
        println!("{}", "disconnected");
    }

    fn join(&self, group_name: &str) -> SpreadGroup {
        SpreadGroup {
            name: String::from_str(group_name)
        }
    }

    fn multicast(
        &self,
        data: &[u8],
        header: SpreadMessageHeader
    ) -> Result<(), SpreadError> {
        Ok(())
    }
}
