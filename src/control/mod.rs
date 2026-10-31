mod client;
mod health;
pub(crate) use health::Health;
mod protocol;
pub(crate) use client::{Client, connected};
pub(crate) use protocol::{Command, read_request, read_response, write_response};

pub(crate) const SOCKET_ENV: &str = "_SOPRA_SERVER_SOCKET";
