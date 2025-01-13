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
use std::io::Error;

use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::dex::meta::auth::RoleRequest;

use crate::common::meta::{
    http::HttpResponse as MetaHttpResponse,
    user::{UserGroup, UserGroupRequest, UserRoleRequest},
};

#[cfg(feature = "enterprise")]
#[post("/{org_id}/roles")]
pub async fn create_role(
    org_id: web::Path<String>,
    user_req: web::Json<UserRoleRequest>,
) -> Result<HttpResponse, Error> {
    use crate::common::meta::user::is_standard_role;

    let org_id = org_id.into_inner();
    let mut user_req = user_req.into_inner();
    user_req.role = user_req.role.trim().to_lowercase();

    if user_req.role.is_empty() || is_standard_role(&user_req.role) {
        return Ok(MetaHttpResponse::bad_request(
            "Role name cannot be empty or standard role",
        ));
    }

    match o2_enterprise::enterprise::openfga::authorizer::roles::create_role(
        &user_req.role,
        &org_id,
    )
    .await
    {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => {
            let err = err.to_string();
            if err.contains("write_failed_due_to_invalid_input") {
                Ok(MetaHttpResponse::bad_request("Role already exists"))
            } else {
                Ok(MetaHttpResponse::internal_error("Something went wrong"))
            }
        }
    }
}

#[cfg(not(feature = "enterprise"))]
#[post("/{org_id}/roles")]
pub async fn create_role(
    _org_id: web::Path<String>,
    _role_id: web::Json<UserRoleRequest>,
) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[delete("/{org_id}/roles/{role_id}")]
pub async fn delete_role(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, role_name) = path.into_inner();

    match o2_enterprise::enterprise::openfga::authorizer::roles::delete_role(&org_id, &role_name)
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => Ok(MetaHttpResponse::internal_error(err)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[delete("/{org_id}/roles/{role_id}")]
pub async fn delete_role(_path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[get("/{org_id}/roles")]
pub async fn get_roles(org_id: web::Path<String>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    let mut permitted;
    // Get List of allowed objects

    let user_id = req.headers().get("user_id").unwrap();
    match crate::handler::http::auth::validator::list_objects_for_user(
        &org_id,
        user_id.to_str().unwrap(),
        "GET",
        "role",
    )
    .await
    {
        Ok(list) => {
            permitted = list;
        }
        Err(e) => {
            return Ok(MetaHttpResponse::forbidden(e.to_string()));
        }
    }
    // Get List of allowed objects ends

    if let Some(mut local_permitted) = permitted {
        let prefix = "role:";
        for value in local_permitted.iter_mut() {
            if value.starts_with(prefix) {
                *value = value.strip_prefix(prefix).unwrap().to_string();
            }
            let role_prefix = format!("{org_id}/");
            if value.starts_with(&role_prefix) {
                *value = value.strip_prefix(&role_prefix).unwrap().to_string()
            }
        }
        permitted = Some(local_permitted);
    }

    match o2_enterprise::enterprise::openfga::authorizer::roles::get_all_roles(&org_id, permitted)
        .await
    {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(err) => Ok(MetaHttpResponse::internal_error(err)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[get("/{org_id}/roles")]
pub async fn get_roles(
    _org_id: web::Path<String>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[put("/{org_id}/roles/{role_id}")]
pub async fn update_role(
    path: web::Path<(String, String)>,
    update_role: web::Json<RoleRequest>,
) -> Result<HttpResponse, Error> {
    let (org_id, role_id) = path.into_inner();
    let update_role = update_role.into_inner();

    match o2_enterprise::enterprise::openfga::authorizer::roles::update_role(
        &org_id,
        &role_id,
        update_role.add,
        update_role.remove,
        update_role.add_users,
        update_role.remove_users,
    )
    .await
    {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(err) => Ok(MetaHttpResponse::internal_error(err)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[post("/{org_id}/roles/{role_id}")]
pub async fn update_role(
    _path: web::Path<(String, String)>,
    _permissions: web::Json<String>,
) -> Result<HttpResponse, actix_web::Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[get("/{org_id}/roles/{role_id}/permissions/{resource}")]
pub async fn get_role_permissions(
    path: web::Path<(String, String, String)>,
) -> Result<HttpResponse, Error> {
    let (org_id, role_id, resource) = path.into_inner();
    match o2_enterprise::enterprise::openfga::authorizer::roles::get_role_permissions(
        &org_id, &role_id, &resource,
    )
    .await
    {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(err) => Ok(MetaHttpResponse::internal_error(err)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[get("/{org_id}/roles/{role_id}/permissions/{resource}")]
pub async fn get_role_permissions(
    _path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[get("/{org_id}/roles/{role_id}/users")]
pub async fn get_users_with_role(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, role_id) = path.into_inner();
    match o2_enterprise::enterprise::openfga::authorizer::roles::get_users_with_role(
        &org_id, &role_id,
    )
    .await
    {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(err) => Ok(MetaHttpResponse::internal_error(err)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[get("/{org_id}/roles/{role_id}/users")]
pub async fn get_users_with_role(_org_id: web::Path<String>) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[get("/{org_id}/users/{user_email}/roles")]
pub async fn get_roles_for_user(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, user_email) = path.into_inner();
    let res = o2_enterprise::enterprise::openfga::authorizer::roles::get_roles_for_org_user(
        &org_id,
        &user_email,
    )
    .await;

    Ok(HttpResponse::Ok().json(res))
}

#[cfg(not(feature = "enterprise"))]
#[get("/{org_id}/users/{user_email}/roles")]
pub async fn get_roles_for_user(_path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[get("/{org_id}/users/{user_email}/groups")]
pub async fn get_groups_for_user(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, user_email) = path.into_inner();
    match o2_enterprise::enterprise::openfga::authorizer::groups::get_groups_for_org_user(
        &org_id,
        &user_email,
    )
    .await
    {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(err) => Ok(MetaHttpResponse::internal_error(err)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[get("/{org_id}/users/{user_email}/groups")]
pub async fn get_groups_for_user(
    _path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[post("/{org_id}/groups")]
pub async fn create_group(
    org_id: web::Path<String>,
    user_group: web::Json<UserGroup>,
) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    let mut user_grp = user_group.into_inner();
    user_grp.name = user_grp.name.trim().to_lowercase();

    match o2_enterprise::enterprise::openfga::authorizer::groups::create_group(
        &org_id,
        &user_grp.name,
        user_grp.users.unwrap_or_default(),
    )
    .await
    {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => Ok(MetaHttpResponse::internal_error(err)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[post("/{org_id}/groups")]
pub async fn create_group(
    _org_id: web::Path<String>,
    _user_group: web::Json<UserGroup>,
) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[put("/{org_id}/groups/{group_name}")]
pub async fn update_group(
    path: web::Path<(String, String)>,
    user_group: web::Json<UserGroupRequest>,
) -> Result<HttpResponse, Error> {
    let (org_id, group_name) = path.into_inner();
    let user_grp = user_group.into_inner();

    match o2_enterprise::enterprise::openfga::authorizer::groups::update_group(
        &org_id,
        &group_name,
        user_grp.add_users,
        user_grp.remove_users,
        user_grp.add_roles,
        user_grp.remove_roles,
    )
    .await
    {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => Ok(MetaHttpResponse::internal_error(err)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[put("/{org_id}/groups/{group_name}")]
pub async fn update_group(
    _org_id: web::Path<String>,
    _user_group: web::Json<UserGroupRequest>,
) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[get("/{org_id}/groups")]
pub async fn get_groups(path: web::Path<String>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();

    let mut permitted;
    // Get List of allowed objects

    let user_id = req.headers().get("user_id").unwrap();
    match crate::handler::http::auth::validator::list_objects_for_user(
        &org_id,
        user_id.to_str().unwrap(),
        "GET",
        "group",
    )
    .await
    {
        Ok(list) => {
            permitted = list;
        }
        Err(e) => {
            return Ok(crate::common::meta::http::HttpResponse::forbidden(
                e.to_string(),
            ));
        }
    }
    // Get List of allowed objects ends

    if let Some(mut local_permitted) = permitted {
        let prefix = "group:";
        for value in local_permitted.iter_mut() {
            if value.starts_with(prefix) {
                *value = value.strip_prefix(prefix).unwrap().to_string();
            }
            let group_prefix = format!("{org_id}/");
            if value.starts_with(&group_prefix) {
                *value = value.strip_prefix(&group_prefix).unwrap().to_string()
            }
        }
        permitted = Some(local_permitted);
    }

    match o2_enterprise::enterprise::openfga::authorizer::groups::get_all_groups(&org_id, permitted)
        .await
    {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(err) => Ok(MetaHttpResponse::internal_error(err)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[get("/{org_id}/groups")]
pub async fn get_groups(_path: web::Path<String>) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[get("/{org_id}/groups/{group_name}")]
pub async fn get_group_details(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, group_name) = path.into_inner();

    match o2_enterprise::enterprise::openfga::authorizer::groups::get_group_details(
        &org_id,
        &group_name,
    )
    .await
    {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(err) => Ok(MetaHttpResponse::internal_error(err)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[get("/{org_id}/groups/{group_name}")]
pub async fn get_group_details(_path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[get("/{org_id}/resources")]
pub async fn get_resources(_org_id: web::Path<String>) -> Result<HttpResponse, Error> {
    use o2_enterprise::enterprise::openfga::meta::mapping::Resource;
    #[cfg(feature = "cloud")]
    use o2_enterprise::enterprise::openfga::meta::mapping::NON_CLOUD_RESOURCE_KEYS;
    let resources = o2_enterprise::enterprise::openfga::meta::mapping::OFGA_MODELS
        .values()
        .collect::<Vec<&Resource>>();
    #[cfg(feature = "cloud")]
    let resources = resources
        .into_iter()
        .filter(|r| !NON_CLOUD_RESOURCE_KEYS.contains(&r.key))
        .collect::<Vec<&Resource>>();
    Ok(HttpResponse::Ok().json(resources))
}

#[cfg(not(feature = "enterprise"))]
#[get("/{org_id}/resources")]
pub async fn get_resources(_org_id: web::Path<String>) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[delete("/{org_id}/groups/{group_name}")]
pub async fn delete_group(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, group_name) = path.into_inner();

    match o2_enterprise::enterprise::openfga::authorizer::groups::delete_group(&org_id, &group_name)
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => Ok(MetaHttpResponse::internal_error(err)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[delete("/{org_id}/groups/{group_name}")]
pub async fn delete_group(_path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    Ok(MetaHttpResponse::forbidden("Not Supported"))
}
