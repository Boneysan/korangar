mod client_info;

use encoding_rs::Encoding;
#[cfg(feature = "debug")]
use korangar_debug::logging::Timer;
use korangar_interface::element::StateElement;
use korangar_loaders::FileLoader;
use quick_xml::Reader;
use quick_xml::de::from_str;
use quick_xml::events::Event;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

pub use self::client_info::{ClientInfo, ClientInfoPathExt, PacketVersion};
use super::GameFileLoader;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, RustState, StateElement)]
pub struct ServiceId(pub usize);

pub fn load_client_info(game_file_loader: &GameFileLoader) -> ClientInfo {
    #[cfg(feature = "debug")]
    let timer = Timer::new("read clientinfo");

    let client_info = game_file_loader
        .get("data\\sclientinfo.xml")
        .or_else(|_| game_file_loader.get("data\\clientinfo.xml"))
        .expect("failed to find clientinfo");

    let content = match get_xml_encoding(&client_info) {
        Some(encoding) => {
            let (cow, _) = encoding.decode_without_bom_handling(&client_info);
            cow
        }
        None => String::from_utf8_lossy(client_info.as_slice()),
    };

    let mut client_info: ClientInfo = from_str(&content).unwrap();
    apply_server_override(&mut client_info, load_server_override());

    #[cfg(feature = "debug")]
    timer.stop();

    client_info
}

/// Where an end user points a build at a server.
///
/// The address otherwise lives in `sclientinfo.xml`, which ships **inside** the
/// release archive, is EUC-KR encoded, and reads `127.0.0.1` — so every copy of
/// a release connects to the machine it is running on. Asking a player to edit
/// XML in a legacy encoding to fix that is not a distribution story.
pub const SERVER_OVERRIDE_PATH: &str = "client/server.ron";

/// An optional `client/server.ron` sitting next to the binary:
///
/// ```ron
/// (
///     address: "ro.example.com",
///     port: 6900,
///     name: "Seal Cascade",
/// )
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct ServerOverride {
    /// Hostname or IP. Resolved at login, so a DNS name is fine.
    pub address: String,
    /// Defaults to the standard login-server port.
    #[serde(default = "default_login_port")]
    pub port: u16,
    /// Optional label for the service picker. Absent keeps whatever
    /// `sclientinfo.xml` named it.
    #[serde(default)]
    pub name: Option<String>,
}

const fn default_login_port() -> u16 {
    6900
}

/// Parses the override file.
///
/// `IMPLICIT_SOME` is the whole reason this is a function rather than a bare
/// `ron::from_str`: without it an `Option` field must be written
/// `name: Some("…")`, and this file is authored by a player who has been handed
/// one line by whoever runs the server. Requiring Rust's `Option` syntax in a
/// config file is a trap, not a format.
///
/// Tests go through here too, deliberately. A test that parsed with different
/// options would be checking a syntax the shipped client rejects.
fn parse_server_override(raw: &str) -> Result<ServerOverride, ron::error::SpannedError> {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
        .from_str(raw)
}

/// Reads the override, and **fails loudly rather than quietly** if it is
/// present and unreadable.
///
/// Ignoring a malformed file would leave the player connected to whatever
/// `sclientinfo.xml` says — `127.0.0.1` in every shipped archive — after they
/// explicitly asked for somewhere else. A connection refused against their own
/// machine is about the least informative symptom available, and this project
/// has spent whole sessions on failures whose only tell was silence.
fn load_server_override() -> Option<ServerOverride> {
    let raw = std::fs::read_to_string(SERVER_OVERRIDE_PATH).ok()?;

    match parse_server_override(&raw) {
        Ok(server_override) => Some(server_override),
        Err(error) => panic!(
            "{SERVER_OVERRIDE_PATH} could not be read: {error}\n\nIt should look like:\n\n(\n    address: \"ro.example.com\",\n    port: \
             6900,\n)\n\nDelete the file to fall back to the address in sclientinfo.xml."
        ),
    }
}

/// Points every service at the override.
///
/// Address and port only: everything else in the entry — packet version, client
/// version, GM account ids, loading screens — is server configuration that a
/// player has no way to know and no business guessing.
fn apply_server_override(client_info: &mut ClientInfo, server_override: Option<ServerOverride>) {
    let Some(server_override) = server_override else {
        return;
    };

    for service in &mut client_info.services {
        service.address = server_override.address.clone();
        service.port = server_override.port;

        if let Some(name) = &server_override.name {
            service.display_name = Some(name.clone());
        }
    }
}

fn get_xml_encoding(data: &[u8]) -> Option<&'static Encoding> {
    let mut reader = Reader::from_reader(data);

    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Decl(xml_declaration)) => {
                if let Some(Ok(encoding)) = xml_declaration.encoding() {
                    return Encoding::for_label(encoding.as_ref());
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => (),
        }
        buffer.clear();
    }

    None
}

#[cfg(test)]
mod server_override_tests {
    use super::*;
    use crate::loaders::server::client_info::Service;

    fn client_info_with(address: &str, port: u16, display: Option<&str>) -> ClientInfo {
        ClientInfo {
            services: vec![Service {
                address: address.to_owned(),
                port,
                display_name: display.map(str::to_owned),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn no_override_leaves_the_shipped_address_alone() {
        let mut client_info = client_info_with("127.0.0.1", 6900, Some("HerculesRO (local)"));
        apply_server_override(&mut client_info, None);

        assert_eq!(client_info.services[0].address, "127.0.0.1");
        assert_eq!(client_info.services[0].port, 6900);
        assert_eq!(client_info.services[0].display_name.as_deref(), Some("HerculesRO (local)"));
    }

    #[test]
    fn an_override_repoints_every_service() {
        let parsed: ServerOverride = parse_server_override(r#"(address: "ro.example.com", port: 5121, name: "Seal Cascade")"#).unwrap();

        let mut client_info = client_info_with("127.0.0.1", 6900, Some("HerculesRO (local)"));
        client_info.services.push(Service {
            address: "127.0.0.1".to_owned(),
            port: 6901,
            ..Default::default()
        });
        apply_server_override(&mut client_info, Some(parsed));

        for service in &client_info.services {
            assert_eq!(service.address, "ro.example.com");
            assert_eq!(service.port, 5121);
        }
        assert_eq!(client_info.services[0].display_name.as_deref(), Some("Seal Cascade"));
    }

    /// The common case: a host hands out one line naming their server.
    #[test]
    fn address_alone_is_enough_and_keeps_the_shipped_name() {
        let parsed: ServerOverride = parse_server_override(r#"(address: "10.0.0.7")"#).unwrap();

        let mut client_info = client_info_with("127.0.0.1", 6900, Some("HerculesRO (local)"));
        apply_server_override(&mut client_info, Some(parsed));

        assert_eq!(client_info.services[0].address, "10.0.0.7");
        assert_eq!(client_info.services[0].port, 6900, "the login port should default, not vanish");
        assert_eq!(
            client_info.services[0].display_name.as_deref(),
            Some("HerculesRO (local)"),
            "omitting `name` must not blank the picker label"
        );
    }

    /// Only the two fields a player can know. Everything else in a service
    /// entry is server configuration.
    #[test]
    fn an_override_does_not_touch_server_configuration() {
        let parsed: ServerOverride = parse_server_override(r#"(address: "ro.example.com")"#).unwrap();

        let mut client_info = client_info_with("127.0.0.1", 6900, None);
        client_info.services[0].version = 55;
        client_info.services[0].packet_version = Some(PacketVersion::_20220406);
        apply_server_override(&mut client_info, Some(parsed));

        assert_eq!(client_info.services[0].version, 55);
        assert!(matches!(client_info.services[0].packet_version, Some(PacketVersion::_20220406)));
    }

    /// The example the release archive ships, parsed by the parser the release
    /// ships — not a copy of it.
    ///
    /// `release.yml` copies this exact file into `client/server.ron.example`.
    /// An example that does not parse is worse than none: it is the first thing
    /// a player edits, and its syntax is what they will copy.
    #[test]
    fn server_override_example_parses() {
        let example = include_str!("../../../server.ron.example");
        let parsed = parse_server_override(example).expect("the shipped example must parse");

        assert_eq!(parsed.address, "ro.example.com");
        assert_eq!(parsed.port, 6900);
        assert_eq!(parsed.name.as_deref(), Some("Seal Cascade"));
    }

    /// A typo must not be read as "no override" — that silently reconnects the
    /// player to 127.0.0.1 after they asked for somewhere else.
    #[test]
    fn a_malformed_override_is_not_mistaken_for_an_absent_one() {
        assert!(parse_server_override(r#"(addres: "ro.example.com")"#).is_err());
        assert!(parse_server_override("ro.example.com:6900").is_err());
    }
}
