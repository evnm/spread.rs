#![crate_id = "spread#0.0.1"]
#![crate_type = "lib"]
#![comment = "A Rust client library for the Spread toolkit"]
#![license = "MIT"]

#[deny(non_camel_case_types)]

use std::io::net::ip::SocketAddr;

mod test;

pub static DefaultSpreadPort: i16 = 4803;
static MaxPrivateNameLength: uint = 10;
static SpreadMajorVersion: u8 = 4;
static SpreadMinorVersion: u8 = 4;
static SpreadPatchVersion: u8 = 0;

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

pub struct SpreadClient {
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

// Construct a byte vector representation of a connect message for the given
// connection arguments.
fn encode_connect_message(
    private_name: &str,
    is_priority_connection: bool,
    receive_membership_messages: bool
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();

    // Set Spread version.
    buf.push(SpreadMajorVersion);
    buf.push(SpreadMinorVersion);
    buf.push(SpreadPatchVersion);

    // Apply masks for group membership and priority.
    let masked = match (is_priority_connection, receive_membership_messages) {
        (true, true) => 1 | 16,
        (true, false) => 1,
        (false, true) => 16,
        (false, false) => 0
    };
    buf.push(masked);

    buf.push(private_name.char_len() as u8);
    buf.push_all(private_name.as_bytes());
    buf
}

/// Establishes a named connection to a Spread daemon running at a given
/// `SocketAddr`.
///
/// *Arguments:*
///
/// - `addr`: The address at which the Spread daemon is running.
/// - `private_name`: A name to use privately to refer to the connection.
/// - `is_priority_connection`: If true, indicates that the connection is
///   prioritized.
/// - `receive_membership_messages`: If true, membership messages will be
///   received by the resultant client.
pub fn connect(
    addr: SocketAddr,
    private_name: &str,
    is_priority_connection: bool,
    receive_membership_messages: bool
) -> SpreadClient {
    // Truncate (if necessary) and write `private_name`.
    let truncated_private_name = match private_name {
        too_long if too_long.char_len() > MaxPrivateNameLength =>
            too_long.slice_to(MaxPrivateNameLength),
        just_fine => just_fine
    };

    // Send the initial connect message.
    let connect_message = encode_connect_message(
        truncated_private_name,
        is_priority_connection,
        receive_membership_messages
    );

    // TODO: Send the connect message.

    SpreadClient {
        addr: addr,
        name: String::from_str(truncated_private_name),
        is_priority_connection: is_priority_connection,
        receive_membership_messages: receive_membership_messages
    }
}

impl SpreadClient {
    /// Disconnects the connection to the Spread daemon.
    pub fn disconnect(&self) {
        println!("{}", "disconnected");
    }

    /// Join a named Spread group on the client's connection.
    /// All messages sent to the group will be received by the client until it
    /// has left the group.
    pub fn join(&self, group_name: &str) -> SpreadGroup {
        SpreadGroup {
            name: String::from_str(group_name)
        }
    }

    /// Send a message to all groups specified in the message header.
    pub fn multicast(
        &self,
        data: &[u8],
        header: SpreadMessageHeader
    ) -> Result<(), SpreadError> {
        Ok(())
    }
}
