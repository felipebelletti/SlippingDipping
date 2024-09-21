use lazy_static::lazy_static;

pub mod broadcast;
pub mod builder;
pub mod types;

use builder::Builder;

lazy_static! {
    // Native support to eob
    pub static ref TITAN: Builder = Builder::new_with_eob("https://rpc.titanbuilder.xyz", "eth_sendEndOfBlockBundle", true);
    pub static ref RSYNC: Builder = Builder::new("https://rsync-builder.xyz/", false);

    // These are supposedly offering eob through the normal "eth_sendBundle" endpoint
    pub static ref BEAVER: Builder = Builder::new("https://rpc.beaverbuild.org/", true);
    pub static ref FLASHBOTS: Builder = Builder::new("https://relay.flashbots.net", true);
    pub static ref BUILDAI: Builder = Builder::new("https://BuildAI.net", true);

    pub static ref BUILDER69: Builder = Builder::new("https://builder0x69.io", false);
    // pub static ref EDENBUILDER: Builder = Builder::new("https://api.edennetwork.io/v1/bundle", true);
    pub static ref ETHBUILDER: Builder = Builder::new("https://eth-builder.com", true);
    pub static ref MANIFOLD: Builder = Builder::new_with_custom_method("https://api.securerpc.com/v1", false, "manifold_sendBundle".to_string());
    pub static ref PAYLOAD: Builder = Builder::new("https://rpc.payload.de", true);
    pub static ref NFACTORIAL: Builder = Builder::new("https://rpc.nfactorial.xyz/", true);
    pub static ref LOKIBUILDER: Builder = Builder::new("https://rpc.lokibuilder.xyz/", false);
    pub static ref PENGUINBUILDER: Builder = Builder::new("https://rpc.penguinbuild.org", false);
    pub static ref BTCS: Builder = Builder::new("https://rpc.btcs.com", false);
    // pub static ref BLOCKBEELDER: Builder = Builder::new("https://blockbeelder.com/rpc", true);
    pub static ref BOBTHEBUILDER: Builder = Builder::new("https://rpc.bobthebuilder.xyz", true);

    pub static ref BUILDERS: Vec<&'static Builder> = vec![
        &*TITAN,
        &*BEAVER,
        &*FLASHBOTS,
        &*BUILDER69,
        &*ETHBUILDER,
        &*MANIFOLD,
        &*BUILDAI,
        &*RSYNC,
        &*NFACTORIAL,
        &*PENGUINBUILDER,
        &*BTCS,
        &*BOBTHEBUILDER
        // &*EDENBUILDER, // removed on purpose, shit ass relayer. use it via flashbots multiplexer instead
        // &*PAYLOAD, // removed on purpose, shit ass relayer. use it via flashbots multiplexer instead
        // &*LOKIBUILDER, // removed on purpose, shit ass relayer. use it via flashbots multiplexer instead
        // &*BLOCKBEELDER,
    ];

    pub static ref PSEUDO_EOB_BUILDERS: Vec<&'static Builder> = vec![
        &*FLASHBOTS,
        &*BUILDER69,
        &*ETHBUILDER,
        &*MANIFOLD,
        &*BUILDAI,
        &*RSYNC,
        &*NFACTORIAL,
        &*PENGUINBUILDER,
        &*BTCS,
        &*BOBTHEBUILDER
        // &*BEAVER, // removed on purpose, we dont want them to be considered a pseudo eob builder
        // &*PAYLOAD, // removed on purpose, shit ass relayer. use it via flashbots multiplexer instead
        // &*EDENBUILDER, // removed on purpose, shit ass relayer. use it via flashbots multiplexer instead
        // &*LOKIBUILDER, // removed on purpose, shit ass relayer. use it via flashbots multiplexer instead
        // &*BLOCKBEELDER, // removed on purpose, shit ass relayer. flashbots multiplexer doesnt even support it
    ];
}
