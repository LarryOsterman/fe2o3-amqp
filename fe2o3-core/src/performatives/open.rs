use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::{Symbol, Uint, Ushort},
};

use crate::definitions::{Fields, IetfLanguageTag, Milliseconds};

#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:open:list",
    code = 0x0000_0000_0000_0010,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Open {
    pub container_id: String,
    pub hostname: Option<String>,
    pub max_frame_size: Uint,
    pub channel_max: Option<Ushort>,
    pub idle_time_out: Option<Milliseconds>,
    pub outgoing_locales: Option<Vec<IetfLanguageTag>>,
    pub incoming_locales: Option<Vec<IetfLanguageTag>>,
    pub offered_capabilities: Option<Vec<Symbol>>,
    pub desired_capabilities: Option<Vec<Symbol>>,
    pub properties: Option<Fields>,
}
