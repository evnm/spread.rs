#![crate_name = "spread"]
#![comment = "A Rust client library for the Spread toolkit"]
#![crate_type = "lib"]
#![license = "MIT"]

#[deny(non_camel_case_types)]

extern crate encoding;

use encoding::{Encoding, EncodeStrict, DecodeStrict};
use encoding::all::ISO_8859_1;
use std::io::{ConnectionFailed, ConnectionRefused, IoError, IoResult, OtherIoError};
use std::io::net::ip::SocketAddr;
use std::io::net::tcp::TcpStream;
use std::result::Result;
use util::{bytes_to_int, flip_endianness, int_to_bytes, same_endianness};

mod test;
mod util;

pub static DefaultSpreadPort: i16 = 4803;

static MaxPrivateNameLength: uint = 10;
static DefaultAuthName: &'static str  = "NULL";
static MaxAuthNameLength: uint = 30;
static MaxAuthMethodCount: uint = 3;
static MaxGroupNameLength: uint = 32;

// Control message types.
// NOTE: The only currently-implemented service type for messaging is "reliable".
enum ControlServiceType {
    JoinMessage     = 0x00010000,
    LeaveMessage    = 0x00020000,
    KillMessage     = 0x00040000,
    ReliableMessage = 0x00000002
}

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

/// A message to be sent or received by a Spread client to/from a group.
pub struct SpreadMessage {
    service_type: u32,
    pub groups: Vec<String>,
    pub sender: String,
    pub data: Vec<u8>,
}

/// Representation of a client connection to a Spread daemon.
pub struct SpreadClient {
    stream: TcpStream,
    pub private_name: String,
    pub groups: Vec<String>,
    receive_membership_messages: bool
}

// Construct a byte vector representation of a connect message for the given
// connection arguments.
fn encode_connect_message(
    private_name: &str,
    receive_membership_messages: bool
) -> Result<Vec<u8>, String> {
    let mut vec: Vec<u8> = Vec::new();

    // Set Spread version.
    vec.push(SpreadMajorVersion);
    vec.push(SpreadMinorVersion);
    vec.push(SpreadPatchVersion);

    // Apply masks for group membership (and priority, which is unimplemented).
    let mask = if receive_membership_messages {
        0x10
    } else {
        0
    };
    vec.push(mask);

    let private_name_buf = try!(ISO_8859_1.encode(private_name, EncodeStrict).map_err(
        |_| format!("Failed to encode private name: {}", private_name)
            ));

    vec.push(private_name.char_len() as u8);
    vec.push_all_move(private_name_buf);
    Ok(vec)
}

/// Establishes a named connection to a Spread daemon running at a given
/// `SocketAddr`.
///
/// *Arguments:*
///
/// - `addr`: The address at which the Spread daemon is running.
/// - `private_name`: A name to use privately to refer to the connection.
/// - `receive_membership_messages`: If true, membership messages will be
///   received by the resultant client.
pub fn connect(
    addr: SocketAddr,
    private_name: &str,
    receive_membership_messages: bool
) -> IoResult<SpreadClient> {
    // Truncate (if necessary) and write `private_name`.
    let truncated_private_name = match private_name {
        too_long if too_long.char_len() > MaxPrivateNameLength =>
            too_long.slice_to(MaxPrivateNameLength),
        just_fine => just_fine
    };

    // Send the initial connect message.
    let connect_message = try!(encode_connect_message(
        truncated_private_name,
        receive_membership_messages
    ).map_err(|error_msg| IoError {
        kind: ConnectionFailed,
        desc: "",
        detail: Some(error_msg)
    }));

    let mut stream = try!(TcpStream::connect(addr.ip.to_string().as_slice(), addr.port));
    try!(stream.write(connect_message.as_slice()));

    // Read the authentication methods.
    let authname_len = try!(stream.read_byte()) as i32;
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
            detail: Some(format!("{}", (0xffffff00 | authname_len as u32) as i32))
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

    for _ in range(authname_len as uint, (MaxAuthNameLength * MaxAuthMethodCount + 1)) {
        authname_vec.push(0);
    }
    try!(stream.write(authname_vec.as_slice()));

    // Check for an accept message.
    let accepted: u8 = try!(stream.read_byte());
    if accepted != AcceptSession as u8 {
        return Err(IoError {
            kind: ConnectionFailed,
            desc: "Connection attempt rejected",
            detail: Some(format!("{}", (0xffffff00 | accepted as u32) as i32))
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
    }

    // Read the private group name.
    let group_name_len = try!(stream.read_byte()) as i32;
    if group_name_len == -1 {
        return Err(IoError {
            kind: ConnectionFailed,
            desc: "Connection closed during connect attempt to read group name length",
            detail: None
        });
    }
    let group_name_buf = try!(stream.read_exact(group_name_len as uint));
    let private_group_name = match String::from_utf8(group_name_buf) {
        Ok(group_name) => group_name,
        Err(error) => return Err(IoError {
            kind: ConnectionFailed,
            desc: "Server sent invalid group name",
            detail: Some(format!("{}", error))
        })
    };

    Ok(SpreadClient {
        stream: stream,
        private_name: private_group_name,
        groups: Vec::new(),
        receive_membership_messages: receive_membership_messages
    })
}

impl SpreadClient {
    // Encode a service message for dispatch to a Spread daemon.
    fn encode_message(
        service_type: u32,
        private_name: &str,
        groups: &[&str],
        data: &[u8]
    ) -> Result<Vec<u8>, String> {
        let mut vec: Vec<u8> = Vec::new();
        vec.push_all_move(int_to_bytes(service_type));

        let private_name_buf = try!(ISO_8859_1.encode(private_name, EncodeStrict).map_err(
            |_| format!("Failed to encode private name: {}", private_name)
        ));
        vec.push_all_move(private_name_buf);
        for _ in range(private_name.len(), (MaxGroupNameLength)) {
            vec.push(0);
        }

        vec.push_all_move(int_to_bytes(groups.len() as u32));
        vec.push_all_move(int_to_bytes(0));
        vec.push_all_move(int_to_bytes(data.len() as u32));

        // Encode and push each group name, converting any encoding errors
        // to error message strings.
        for group in groups.iter() {
            let group_buf = try!(ISO_8859_1.encode(*group, EncodeStrict).map_err(
                |_| format!("Failed to encode group name: {}", group)
            ));
            vec.push_all_move(group_buf);
            for _ in range(group.len(), (MaxGroupNameLength)) {
                vec.push(0);
            }
        }

        vec.push_all(data);
        Ok(vec)
    }

    /// Disconnects the client from the Spread daemon.
    // TODO: Prevent further usage of client?
    pub fn disconnect(&mut self) -> IoResult<()> {
        let name_slice = self.private_name.as_slice();
        let kill_message = try!(SpreadClient::encode_message(
            KillMessage as u32,
            name_slice,
            [name_slice],
            []
        ).map_err(|error_msg| IoError {
            kind: OtherIoError,
            desc: "Disconnection failed",
            detail: Some(error_msg)
        }));

        self.stream.write(kill_message.as_slice())
    }

    /// Join a named Spread group.
    ///
    /// All messages sent to the group will be received by the client until it
    /// has left the group.
    pub fn join(&mut self, group_name: &str) -> IoResult<()> {
        let join_message = try!(SpreadClient::encode_message(
            JoinMessage as u32,
            self.private_name.as_slice(),
            [group_name],
            []
        ).map_err(|error_msg| IoError {
            kind: OtherIoError,
            desc: "Disconnection failed",
            detail: Some(error_msg)
        }));

        try!(self.stream.write(join_message.as_slice()));
        self.groups.push(group_name.to_string());
        Ok(())
    }

    /// Leave a named Spread group.
    pub fn leave(&mut self, group_name: &str) -> IoResult<()> {
        let leave_message = try!(SpreadClient::encode_message(
            LeaveMessage as u32,
            self.private_name.as_slice(),
            [group_name],
            []
        ).map_err(|error_msg| IoError {
            kind: OtherIoError,
            desc: "Disconnection failed",
            detail: Some(error_msg)
        }));

        try!(self.stream.write(leave_message.as_slice()));
        self.groups.push(group_name.to_string());
        Ok(())
    }


    /// Send a message to a set of named groups.
    pub fn multicast(
        &mut self,
        groups: &[&str],
        data: &[u8]
    ) -> IoResult<()> {
        let message = try!(SpreadClient::encode_message(
            ReliableMessage as u32,
            self.private_name.as_slice(),
            groups,
            data
        ).map_err(|error_msg| IoError {
            kind: OtherIoError,
            desc: "Disconnection failed",
            detail: Some(error_msg)
        }));

        self.stream.write(message.as_slice())
    }

    /// Receive the next available message. If there are no messages available,
    /// the call will block until either a message is received or a timeout
    /// expires.
    pub fn receive(&mut self) -> IoResult<SpreadMessage> {
        // Header format (sizes in bytes):
        //   svc_type:   4
        //   sender:    32
        //   num_groups: 4
        //   hint:       4
        //   data_len:   4
        let header_vec = try!(self.stream.read_exact(MaxGroupNameLength + 16));
        let is_correct_endianness = same_endianness(bytes_to_int(header_vec.slice(0, 4)));

        let svc_type = match (is_correct_endianness, bytes_to_int(header_vec.slice(0, 4))) {
            (true, correct) => correct,
            (false, incorrect) => flip_endianness(incorrect)
        };

        let sender = try!(
            ISO_8859_1.decode(header_vec.slice(4, 36), DecodeStrict).map_err(|error| IoError {
                kind: OtherIoError,
                desc: "Decoding sender name failed",
                detail: Some(String::from_str(error.as_slice()))
            })
        );

        let num_groups = match (is_correct_endianness, bytes_to_int(header_vec.slice(36, 40))) {
            (true, correct) => correct,
            (false, incorrect) => flip_endianness(incorrect)
        };
        let data_len = match (is_correct_endianness, bytes_to_int(header_vec.slice(44, 48))) {
            (true, correct) => correct,
            (false, incorrect) => flip_endianness(incorrect)
        };

        // Groups format (sizes in bytes):
        //   groups: num_groups
        let groups_vec =
            try!(self.stream.read_exact(MaxGroupNameLength * num_groups as uint));
        let mut groups = Vec::new();

        for n in range(0, num_groups) {
            let i: uint = n as uint * MaxGroupNameLength;
            let group = try!(
                ISO_8859_1.decode(groups_vec.slice(i, i + MaxGroupNameLength), DecodeStrict)
                    .map_err(|error| IoError {
                        kind: OtherIoError,
                        desc: "Decoding group name failed",
                        detail: Some(String::from_str(error.as_slice()))
                    }));
            groups.push(group);
        }

        // Data format (sizes in bytes):
        //   data: data_len
        let data_vec = try!(self.stream.read_exact(data_len as uint));

        Ok(SpreadMessage {
            service_type: svc_type as u32,
            groups: groups,
            sender: sender,
            data: data_vec
        })
    }
}
