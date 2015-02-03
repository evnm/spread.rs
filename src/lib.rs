#![crate_name = "spread"]
#![crate_type = "lib"]
#![feature(collections)]
#![feature(core)]
#![feature(io)]

#[deny(non_camel_case_types)]

extern crate encoding;
#[macro_use] extern crate log;

use encoding::{Encoding, EncoderTrap, DecoderTrap};
use encoding::all::ISO_8859_1;
use std::old_io::{ConnectionFailed, ConnectionRefused, IoError, IoResult, OtherIoError};
use std::old_io::net::ip::ToSocketAddr;
use std::old_io::net::tcp::TcpStream;
use std::result::Result;
use util::{bytes_to_int, flip_endianness, int_to_bytes, same_endianness};

mod test;
mod util;

pub static DEFAULT_SPREAD_PORT: i16 = 4803;

static MAX_PRIVATE_NAME_LENGTH: usize = 10;
static DEFAULT_AUTH_NAME: &'static str  = "NULL";
static MAX_AUTH_NAME_LENGTH: usize = 30;
static MAX_AUTH_METHOD_COUNT: usize = 3;
static MAX_GROUP_NAME_LENGTH: usize = 32;

// Control message types.
// NOTE: The only currently-implemented service type for messaging is "reliable".
enum ControlServiceType {
    JoinMessage     = 0x00010000,
    LeaveMessage    = 0x00020000,
    KillMessage     = 0x00040000,
    ReliableMessage = 0x00000002
}

static SPREAD_MAJOR_VERSION: u8 = 4;
static SPREAD_MINOR_VERSION: u8 = 4;
static SPREAD_PATCH_VERSION: u8 = 0;

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

impl Copy for SpreadError {}

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
    vec.push(SPREAD_MAJOR_VERSION);
    vec.push(SPREAD_MINOR_VERSION);
    vec.push(SPREAD_PATCH_VERSION);

    // Apply masks for group membership (and priority, which is unimplemented).
    let mask = if receive_membership_messages {
        0x10
    } else {
        0
    };
    vec.push(mask);

    let private_name_buf = try!(ISO_8859_1.encode(private_name, EncoderTrap::Strict).map_err(
        |_| format!("Failed to encode private name: {}", private_name)
    ));

    vec.push(private_name.len() as u8);
    vec.push_all(private_name_buf.as_slice());
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
pub fn connect<A: ToSocketAddr>(
    addr: A,
    private_name: &str,
    receive_membership_messages: bool
) -> IoResult<SpreadClient> {
    // Truncate (if necessary) and write `private_name`.
    let truncated_private_name = match private_name {
        too_long if too_long.len() > MAX_PRIVATE_NAME_LENGTH =>
            &too_long[..MAX_PRIVATE_NAME_LENGTH],
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

    let socket_addr = try!(addr.to_socket_addr());
    let mut stream = try!(TcpStream::connect(socket_addr));
    debug!("Sending connect message to {}", socket_addr);
    try!(stream.write_all(connect_message.as_slice()));

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
    let authname_vec = try!(stream.read_exact(authname_len as usize));
    let authname = try!(ISO_8859_1.decode(
        authname_vec.as_slice(), DecoderTrap::Strict
    ).map_err(|error| IoError {
        kind: OtherIoError,
        desc: "Failed to decode received authname",
        detail: Some(String::from_str(&error))
    }));
    debug!("Received authentication method choice(s): {}", authname);

    // Send auth method choice.
    let mut authname_vec: Vec<u8> = match ISO_8859_1.encode(DEFAULT_AUTH_NAME, EncoderTrap::Strict) {
        Ok(vec) => vec,
        Err(error) => return Err(IoError {
            kind: ConnectionFailed,
            desc: "Failed to encode authname",
            detail: Some(format!("{}", error))
        })
    };

    for _ in range(authname_len as usize, (MAX_AUTH_NAME_LENGTH * MAX_AUTH_METHOD_COUNT + 1)) {
        authname_vec.push(0);
    }

    debug!("Sending authentication method choice of {}", DEFAULT_AUTH_NAME);
    try!(stream.write_all(authname_vec.as_slice()));

    // Check for an accept message.
    let accepted: u8 = try!(stream.read_byte());
    if accepted != SpreadError::AcceptSession as u8 {
        return Err(IoError {
            kind: ConnectionFailed,
            desc: "Connection attempt rejected",
            detail: Some(format!("{}", (0xffffff00 | accepted as u32) as i32))
        });
    }

    debug!("Received session acceptance message from daemon");

    // Read the version of Spread that the server is running.
    let (major, minor, patch) =
        (try!(stream.read_byte()) as i32,
         try!(stream.read_byte()) as i32,
         try!(stream.read_byte()) as i32);

    debug!(
        "Received version message: daemon running Spread version {}.{}.{}",
        major, minor, patch
    );

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
    let group_name_buf = try!(stream.read_exact(group_name_len as usize));
    let private_group_name = match String::from_utf8(group_name_buf) {
        Ok(group_name) => group_name,
        Err(error) => return Err(IoError {
            kind: ConnectionFailed,
            desc: "Server sent invalid group name",
            detail: Some(format!("{}", error))
        })
    };

    debug!("Received private name assignment from daemon: {}", private_group_name);
    debug!("Client connected to daemon at {}", socket_addr);

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
        vec.push_all(int_to_bytes(service_type).as_slice());

        let private_name_buf = try!(ISO_8859_1.encode(private_name, EncoderTrap::Strict).map_err(
            |_| format!("Failed to encode private name: {}", private_name)
        ));
        vec.push_all(private_name_buf.as_slice());
        for _ in range(private_name.len(), (MAX_GROUP_NAME_LENGTH)) {
            vec.push(0);
        }

        vec.push_all(int_to_bytes(groups.len() as u32).as_slice());
        vec.push_all(int_to_bytes(0).as_slice());
        vec.push_all(int_to_bytes(data.len() as u32).as_slice());

        // Encode and push each group name, converting any encoding errors
        // to error message strings.
        for group in groups.iter() {
            let group_buf = try!(ISO_8859_1.encode(*group, EncoderTrap::Strict).map_err(
                |_| format!("Failed to encode group name: {}", group)
            ));
            vec.push_all(group_buf.as_slice());
            for _ in range(group.len(), (MAX_GROUP_NAME_LENGTH)) {
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
            ControlServiceType::KillMessage as u32,
            name_slice,
            [name_slice].as_slice(),
            [].as_slice()
        ).map_err(|error_msg| IoError {
            kind: OtherIoError,
            desc: "Disconnection failed",
            detail: Some(error_msg)
        }));

        debug!("Disconnecting from daemon at {}", try!(self.stream.peer_name()));
        self.stream.write_all(kill_message.as_slice())
    }

    /// Join a named Spread group.
    ///
    /// All messages sent to the group will be received by the client until it
    /// has left the group.
    pub fn join(&mut self, group_name: &str) -> IoResult<()> {
        let join_message = try!(SpreadClient::encode_message(
            ControlServiceType::JoinMessage as u32,
            self.private_name.as_slice(),
            [group_name].as_slice(),
            [].as_slice()
        ).map_err(|error_msg| IoError {
            kind: OtherIoError,
            desc: "Group join failed",
            detail: Some(error_msg)
        }));

        debug!("Client \"{}\" joining group \"{}\"", self.private_name, group_name);
        try!(self.stream.write_all(join_message.as_slice()));
        self.groups.push(group_name.to_string());
        Ok(())
    }

    /// Leave a named Spread group.
    pub fn leave(&mut self, group_name: &str) -> IoResult<()> {
        let leave_message = try!(SpreadClient::encode_message(
            ControlServiceType::LeaveMessage as u32,
            self.private_name.as_slice(),
            [group_name].as_slice(),
            [].as_slice()
        ).map_err(|error_msg| IoError {
            kind: OtherIoError,
            desc: "Group leave failed",
            detail: Some(error_msg)
        }));

        debug!("Client \"{}\" leaving group \"{}\"", self.private_name, group_name);
        try!(self.stream.write_all(leave_message.as_slice()));
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
            ControlServiceType::ReliableMessage as u32,
            self.private_name.as_slice(),
            groups,
            data
        ).map_err(|error_msg| IoError {
            kind: OtherIoError,
            desc: "Multicast failed",
            detail: Some(error_msg)
        }));

        debug!("Client \"{}\" multicasting {} bytes to group(s) {:?}",
               self.private_name, data.len(), groups);
        self.stream.write_all(message.as_slice())
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
        let header_vec = try!(self.stream.read_exact(MAX_GROUP_NAME_LENGTH + 16));
        let is_correct_endianness = same_endianness(bytes_to_int(&header_vec[0..4]));

        let svc_type = match (is_correct_endianness, bytes_to_int(&header_vec[0..4])) {
            (true, correct) => correct,
            (false, incorrect) => flip_endianness(incorrect)
        };

        let sender = try!(
            ISO_8859_1.decode(
                &header_vec[4..36],
                DecoderTrap::Strict
            ).map_err(|error| IoError {
                kind: OtherIoError,
                desc: "Failed to decode sender name",
                detail: Some(String::from_str(&error))
            })
        );

        let num_groups = match (is_correct_endianness, bytes_to_int(&header_vec[36..40])) {
            (true, correct) => correct,
            (false, incorrect) => flip_endianness(incorrect)
        };
        let data_len = match (is_correct_endianness, bytes_to_int(&header_vec[44..48])) {
            (true, correct) => correct,
            (false, incorrect) => flip_endianness(incorrect)
        };

        // Groups format (sizes in bytes):
        //   groups: num_groups
        let groups_vec =
            try!(self.stream.read_exact(MAX_GROUP_NAME_LENGTH * num_groups as usize));
        let mut groups = Vec::new();

        for n in range(0, num_groups) {
            let i: usize = n as usize * MAX_GROUP_NAME_LENGTH;
            let group = try!(
                ISO_8859_1.decode(&groups_vec[i..i + MAX_GROUP_NAME_LENGTH], DecoderTrap::Strict)
                    .map_err(|error| IoError {
                        kind: OtherIoError,
                        desc: "Failed to decode group name",
                        detail: Some(String::from_str(&error))
                    }));
            groups.push(group);
        }

        // Data format (sizes in bytes):
        //   data: data_len
        let data_vec = try!(self.stream.read_exact(data_len as usize));

        debug!("Received {} bytes from \"{}\" sent to group(s) {:?}",
               data_len, sender, groups);

        Ok(SpreadMessage {
            service_type: svc_type as u32,
            groups: groups,
            sender: sender,
            data: data_vec
        })
    }
}
