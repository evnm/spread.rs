#![crate_name = "spread"]
#![comment = "A Rust client library for the Spread toolkit"]
#![crate_type = "lib"]
#![license = "MIT"]

#[deny(non_camel_case_types)]

extern crate encoding;

use encoding::{Encoding, EncodeStrict};
use encoding::all::ISO_8859_1;
use std::io::net::ip::SocketAddr;
use std::io::net::tcp::TcpStream;
use std::io::{ConnectionFailed, ConnectionRefused, IoError};
use std::result::Result;

mod test;

pub static DefaultSpreadPort: i16 = 4803;

static MaxPrivateNameLength: uint = 10;
static DefaultAuthName: &'static str  = "NULL";
static MaxAuthNameLength: uint = 30;
static MaxAuthMethodCount: uint = 3;

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
    stream: TcpStream,
    name: String,
    group: SpreadGroup,
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
) -> Result<SpreadClient, IoError> {
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

    let mut stream = try!(TcpStream::connect(addr.ip.to_string().as_slice(), addr.port));
    try!(stream.write(connect_message.as_slice()));

    // Read the authentication methods.
    let authname_len: u8 = try!(stream.read_byte());
    if authname_len == -1 {
        return Err(IoError {
            kind: ConnectionFailed,
            desc: "Connection closed during connect attempt to read auth name length",
            detail: None
        });
    } else if authname_len >= 128 {
        return Err(IoError {
            kind: ConnectionRefused,
            desc: "Connection attempt rejected",
            detail: Some(format!("{}", (-256 as i32 | authname_len as i32)))
        });
    }

    // Ignore the list.
    // TODO: Support IP-based auth?
    try!(stream.read_exact(authname_len as uint));

    // Send auth method choice.
    let mut authname_vec: Vec<u8> = match ISO_8859_1.encode(DefaultAuthName, EncodeStrict) {
        Ok(vec) => vec,
        Err(error) => return Err(IoError {
            kind: ConnectionFailed,
            desc: "Failed to encode authname",
            detail: Some(format!("{}", error))
        })
    };

    for _ in range(authname_len, (MaxAuthNameLength * MaxAuthMethodCount + 1) as u8) {
        authname_vec.push(0);
    }
    try!(stream.write(authname_vec.as_slice()));

    // Check for an accept message.
    let accepted: u8 = try!(stream.read_byte());
    if accepted != AcceptSession as u8 {
        return Err(IoError {
            kind: ConnectionFailed,
            desc: "Connection attempt rejected",
            detail: Some(format!("{}", (-256 as i32 | accepted as i32)))
        });
    }

    // Read the version of Spread that the server is running.
    let (major, minor, patch) =
        (try!(stream.read_byte()) as i32, try!(stream.read_byte()) as i32, try!(stream.read_byte()) as i32);

    if major == -1 || minor == -1 || patch == -1 {
        return Err(IoError {
            kind: ConnectionFailed,
            desc: "Invalid version returned from server",
            detail: Some(format!("{}.{}.{}", major, minor, patch))
        });
    }

    let version_sum = (major*10000) + (minor*100) + patch;
    if version_sum < 30100 {
        return Err(IoError {
            kind: ConnectionFailed,
            desc: "Server is running old, unsupported version of Spread",
            detail: Some(format!("{}.{}.{}", major, minor, patch))
        });
    } else if version_sum < 30800 && is_priority_connection {
        return Err(IoError {
            kind: ConnectionFailed,
            desc: "Server is running old version of Spread that does not support priority connections",
            detail: Some(format!("{}.{}.{}", major, minor, patch))
        });
    }

    // Read the private group name.
    let group_name_len: u8 = try!(stream.read_byte());
    if group_name_len == -1 {
        return Err(IoError {
            kind: ConnectionFailed,
            desc: "Connection closed during connect attempt to read group name length",
            detail: None
        });
    }
    let group_name_buf = try!(stream.read_exact(group_name_len as uint));
    let group_name = match String::from_utf8(group_name_buf) {
        Ok(group_name) => group_name,
        Err(error) => return Err(IoError {
            kind: ConnectionFailed,
            desc: "Server sent invalid group name",
            detail: Some(format!("{}", error))
        })
    };

    Ok(SpreadClient {
        stream: stream,
        name: String::from_str(truncated_private_name),
        group: SpreadGroup { name: group_name },
        is_priority_connection: is_priority_connection,
        receive_membership_messages: receive_membership_messages
    })
}

/*
impl SpreadClient {
    /// Disconnects the connection to the Spread daemon.
    pub fn disconnect(&self)

    /// Join a named Spread group on the client's connection.
    /// All messages sent to the group will be received by the client until it
    /// has left the group.
    pub fn join(&self, group_name: &str) -> SpreadGroup

    /// Send a message to all groups specified in the message header.
    pub fn multicast(
        &self,
        data: &[u8],
        header: SpreadMessageHeader
    ) -> Result<(), SpreadError>
}
 */
