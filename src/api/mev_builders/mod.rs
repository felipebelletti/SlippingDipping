use lazy_static::lazy_static;

pub mod builder;
pub mod types;
pub mod broadcast;

use builder::Builder;

lazy_static! {
    // Native support to eob
    pub static ref TITAN: Builder = Builder::new_with_eob("https://rpc.titanbuilder.xyz", "eth_sendEndOfBlockBundle");
    pub static ref RSYNC: Builder = Builder::new_with_eob("https://rsync-builder.xyz/", "eth_sendBackrunBlockBundle");

    // These are supposedly offering eob through the normal "eth_sendBundle" endpoint
    pub static ref BEAVER: Builder = Builder::new_with_eob("https://rpc.beaverbuild.org/", "eth_sendBundle");
    pub static ref FLASHBOTS: Builder = Builder::new_with_eob("https://relay.flashbots.net", "eth_sendBundle");
    pub static ref BUILDAI: Builder = Builder::new_with_eob("https://BuildAI.net", "eth_sendBundle");

    pub static ref BUILDER69: Builder = Builder::new("https://builder0x69.io");
    pub static ref EDENBUILDER: Builder = Builder::new("https://api.edennetwork.io/v1/bundle");
    pub static ref ETHBUILDER: Builder = Builder::new("https://eth-builder.com");
    pub static ref LIGHTSPEEDBUILDER: Builder = Builder::new("https://rpc.lightspeedbuilder.info/");
    pub static ref MANIFOLD: Builder = Builder::new("https://api.securerpc.com/v1");
    pub static ref PAYLOAD: Builder = Builder::new("https://rpc.payload.de");
    pub static ref NFACTORIAL: Builder = Builder::new("https://rpc.nfactorial.xyz/");
    pub static ref LOKIBUILDER: Builder = Builder::new("https://rpc.lokibuilder.xyz/");
    pub static ref PENGUINBUILDER: Builder = Builder::new("https://rpc.penguinbuild.org");
    pub static ref BTCS: Builder = Builder::new("https://rpc.btcs.com");
    pub static ref BLOCKBEELDER: Builder = Builder::new("https://blockbeelder.com/rpc");
    pub static ref BOBTHEBUILDER: Builder = Builder::new("https://rpc.bobthebuilder.xyz");
    pub static ref BUILDERS: Vec<&'static Builder> = vec![
        &*TITAN,
        &*BEAVER,
        &*FLASHBOTS,
        &*BUILDER69,
        &*EDENBUILDER,
        &*ETHBUILDER,
        &*LIGHTSPEEDBUILDER,
        &*MANIFOLD,
        &*BUILDAI,
        &*PAYLOAD,
        &*RSYNC,
        &*NFACTORIAL,
        &*LOKIBUILDER,
        &*PENGUINBUILDER,
        &*BTCS,
        &*BLOCKBEELDER,
        &*BOBTHEBUILDER
    ];
}