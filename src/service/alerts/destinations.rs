// Copyright 2024 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use actix_web::http;
use config::meta::alerts::destinations::{Destination, DestinationType, DestinationWithTemplate};

use crate::{
    common::{
        infra::config::STREAM_ALERTS,
        meta::authz::Authz,
        utils::auth::{is_ofga_unsupported, remove_ownership, set_ownership},
    },
    service::db::{self, user},
};

pub async fn save(
    org_id: &str,
    name: &str,
    mut destination: Destination,
    create: bool,
) -> Result<(), (http::StatusCode, anyhow::Error)> {
    // First validate the `destination` according to its `destination_type`
    match destination.destination_type {
        DestinationType::Http => {
            if destination.url.is_empty() {
                return Err((
                    http::StatusCode::BAD_REQUEST,
                    anyhow::anyhow!("Alert destination URL needs to be specified"),
                ));
            }
        }
        DestinationType::Email => {
            if destination.emails.is_empty() {
                return Err((
                    http::StatusCode::BAD_REQUEST,
                    anyhow::anyhow!("Atleast one alert destination email is required"),
                ));
            }
            let mut lowercase_emails = vec![];
            for email in destination.emails.iter() {
                let email = email.trim().to_lowercase();
                // Check if the email is part of the org
                match user::get(Some(org_id), &email).await {
                    Ok(user) => {
                        if user.is_none() {
                            return Err((
                                http::StatusCode::BAD_REQUEST,
                                anyhow::anyhow!("Destination email must be part of this org"),
                            ));
                        }
                    }
                    Err(_) => {
                        return Err((
                            http::StatusCode::BAD_REQUEST,
                            anyhow::anyhow!("Destination email must be part of this org"),
                        ));
                    }
                }
                lowercase_emails.push(email);
            }
            if !config::get_config().smtp.smtp_enabled {
                return Err((
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    anyhow::anyhow!("SMTP not configured"),
                ));
            }
            destination.emails = lowercase_emails;
        }
        DestinationType::Sns => {
            if destination.sns_topic_arn.is_none() || destination.aws_region.is_none() {
                return Err((
                    http::StatusCode::BAD_REQUEST,
                    anyhow::anyhow!("Topic ARN and Region are required for SNS destinations"),
                ));
            }
        }
        _ => {}
    }

    if !name.is_empty() {
        destination.name = name.to_string();
    }
    destination.name = destination.name.trim().to_string();
    // Don't allow the characters not supported by ofga
    if is_ofga_unsupported(&destination.name) {
        return Err((
            http::StatusCode::BAD_REQUEST,
            anyhow::anyhow!(
                "Alert destination name cannot contain ':', '#', '?', '&', '%', quotes and space characters"
            ),
        ));
    }
    if destination.name.is_empty() {
        return Err((
            http::StatusCode::BAD_REQUEST,
            anyhow::anyhow!("Alert destination name is required"),
        ));
    }
    if destination.name.contains('/') {
        return Err((
            http::StatusCode::BAD_REQUEST,
            anyhow::anyhow!("Alert destination name cannot contain '/'"),
        ));
    }

    if destination
        .destination_type
        .ne(&DestinationType::HttpNoTemplate)
        && db::alerts::templates::get(org_id, &destination.template)
            .await
            .is_err()
    {
        return Err((
            http::StatusCode::BAD_REQUEST,
            anyhow::anyhow!("Alert template {} not found", destination.template),
        ));
    }

    match db::alerts::destinations::get(org_id, &destination.name).await {
        Ok(_) => {
            if create {
                return Err((
                    http::StatusCode::BAD_REQUEST,
                    anyhow::anyhow!("Alert destination already exists"),
                ));
            }
        }
        Err(_) => {
            if !create {
                return Err((
                    http::StatusCode::BAD_REQUEST,
                    anyhow::anyhow!("Alert destination not found"),
                ));
            }
        }
    }

    match db::alerts::destinations::set(org_id, &destination).await {
        Ok(_) => {
            if name.is_empty() {
                set_ownership(org_id, "destinations", Authz::new(&destination.name)).await;
            }
            Ok(())
        }
        Err(e) => Err((http::StatusCode::BAD_REQUEST, e)),
    }
}

pub async fn get(org_id: &str, name: &str) -> Result<Destination, anyhow::Error> {
    db::alerts::destinations::get(org_id, name)
        .await
        .map_err(|_| anyhow::anyhow!("Alert destination not found"))
}

pub async fn get_with_template(
    org_id: &str,
    name: &str,
) -> Result<DestinationWithTemplate, anyhow::Error> {
    let dest = get(org_id, name).await?;
    let template = db::alerts::templates::get(org_id, &dest.template).await?;
    Ok(dest.with_template(template))
}

pub async fn list(
    org_id: &str,
    permitted: Option<Vec<String>>,
) -> Result<Vec<Destination>, anyhow::Error> {
    match db::alerts::destinations::list(org_id).await {
        Ok(destinations) => {
            let mut result = Vec::new();
            for dest in destinations {
                if permitted.is_none()
                    || permitted
                        .as_ref()
                        .unwrap()
                        .contains(&format!("destination:{}", dest.name))
                    || permitted
                        .as_ref()
                        .unwrap()
                        .contains(&format!("destination:_all_{}", org_id))
                {
                    result.push(dest);
                }
            }
            Ok(result)
        }
        Err(e) => Err(e),
    }
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), (http::StatusCode, anyhow::Error)> {
    let cacher = STREAM_ALERTS.read().await;
    for (stream_key, alerts) in cacher.iter() {
        for alert in alerts.iter() {
            if stream_key.starts_with(org_id) && alert.destinations.contains(&name.to_string()) {
                return Err((
                    http::StatusCode::CONFLICT,
                    anyhow::anyhow!("Alert destination is in use for alert {}", alert.name),
                ));
            }
        }
    }
    drop(cacher);

    if db::alerts::destinations::get(org_id, name).await.is_err() {
        return Err((
            http::StatusCode::NOT_FOUND,
            anyhow::anyhow!("Alert destination not found {}", name),
        ));
    }

    match db::alerts::destinations::delete(org_id, name).await {
        Ok(_) => {
            remove_ownership(org_id, "destinations", Authz::new(name)).await;
            Ok(())
        }
        Err(e) => Err((http::StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}
