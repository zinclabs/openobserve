use o2_enterprise::enterprise::cipher::http_repr::HttpKey;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod registry;

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct KeyAddRequest {
    pub name: String,
    pub key: HttpKey,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct KeyGetResponse {
    pub name: String,
    pub key: HttpKey,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct KeyInfo {
    pub name: String,
    pub key: HttpKey,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct KeyListResponse {
    pub keys: Vec<KeyInfo>,
}
