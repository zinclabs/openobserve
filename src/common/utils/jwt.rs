#[cfg(feature = "enterprise")]
use {
    jsonwebtoken::{
        decode, decode_header,
        jwk::{self, AlgorithmParameters},
        Algorithm, DecodingKey, TokenData, Validation,
    },
    serde_json::Value,
    std::{collections::HashMap, str::FromStr},
};

#[cfg(feature = "enterprise")]
use crate::common::meta::user::TokenValidationResponse;

#[cfg(feature = "enterprise")]
pub(crate) async fn verify_decode_token(
    token: &str,
    jwks: &str,
    aud: &str,
    get_decode_token: bool,
    login_flow: bool,
) -> Result<
    (
        TokenValidationResponse,
        Option<TokenData<HashMap<String, Value>>>,
    ),
    anyhow::Error,
> {
    use config::meta::user::UserRole;
    use infra::errors::JwtError;

    let jwks: jwk::JwkSet = serde_json::from_str(jwks).unwrap();
    let header = decode_header(token)?;
    let kid = match header.kid {
        Some(k) => k,
        None => return Err(JwtError::MissingAttribute("`kid` header".to_owned()).into()),
    };

    if let Some(j) = jwks.find(&kid) {
        match &j.algorithm {
            AlgorithmParameters::RSA(rsa) => {
                let decoding_key = DecodingKey::from_rsa_components(&rsa.n, &rsa.e).unwrap();

                let mut validation = Validation::new(
                    Algorithm::from_str(j.common.key_algorithm.unwrap().to_string().as_str())
                        .unwrap(),
                );
                if login_flow {
                    validation.validate_exp = true;
                    validation.set_audience(&[aud]);
                } else {
                    // we are decoding the token for the service account, which is issued by the dex
                    // hence we don't need to validate the exp and aud
                    validation.validate_exp = false;
                    validation.validate_aud = false;
                };
                let decoded_token = decode::<HashMap<String, serde_json::Value>>(
                    token,
                    &decoding_key,
                    &validation,
                )?;
                let mut final_claims = HashMap::new();
                let claims = decoded_token.clone().claims;
                if let Some(federated_claims) = claims.get("federated_claims") {
                    if let Some(map) = federated_claims.as_object() {
                        for (key, value) in map.iter() {
                            final_claims.insert(key.to_string(), value.clone());
                        }
                    }
                };

                final_claims.extend(claims);

                let user_email = if let Some(email) = final_claims.get("email") {
                    email.as_str().unwrap().to_lowercase()
                } else if let Some(user_id) = final_claims.get("user_id") {
                    user_id.as_str().unwrap().to_lowercase()
                } else {
                    "".to_string()
                };

                let user_name = if let Some(name) = final_claims.get("name") {
                    name.as_str().unwrap()
                } else {
                    ""
                };

                let family_name = if let Some(family_name) = final_claims.get("family_name") {
                    family_name.as_str().unwrap()
                } else {
                    ""
                };

                let given_name = if let Some(given_name) = final_claims.get("given_name") {
                    given_name.as_str().unwrap()
                } else {
                    ""
                };
                let user_role = if login_flow {
                    None
                } else {
                    Some(UserRole::ServiceAccount)
                };

                Ok((
                    TokenValidationResponse {
                        is_valid: true,
                        user_email,
                        user_name: user_name.to_owned(),
                        family_name: family_name.to_owned(),
                        given_name: given_name.to_owned(),
                        is_internal_user: false,
                        user_role,
                    },
                    if get_decode_token {
                        Some(decoded_token)
                    } else {
                        None
                    },
                ))
            }
            _ => Err(JwtError::ValidationFailed().into()),
        }
    } else {
        Err(JwtError::KeyNotExists().into())
    }
}
