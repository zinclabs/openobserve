import http from "./http";

// ----------- Groups -------------
export const getGroups = (org_identifier: string) => {
  const url = `/api/${org_identifier}/groups`;
  return http().get(url);
};

export const getGroup = (group_name: string, org_identifier: string) => {
  const url = `/api/${org_identifier}/groups/${group_name}`;
  return http().get(url);
};

export const deleteGroup = (group_name: string, org_identifier: string) => {
  const url = `/api/${org_identifier}/groups/${group_name}`;
  return http().delete(url);
};

export const createGroup = (group_name: string, org_identifier: string) => {
  const url = `/api/${org_identifier}/groups`;
  return http({}).post(url, {
    name: group_name,
    users: [],
    roles: [],
  });
};

export const updateGroup = (group: {
  group_name: string;
  org_identifier: string;
  payload: {
    add_roles: string[];
    remove_roles: string[];
    add_users: string[];
    remove_users: string[];
  };
}) => {
  const url = `/api/${group.org_identifier}/groups/${group.group_name}`;
  return http().put(url, group.payload);
};

// ----------- Roles -------------

export const getRoles = (org_identifier: string) => {
  const url = `/api/${org_identifier}/roles`;
  return http().get(url);
};

export const getRole = (role_id: string, org_identifier: string) => {
  const url = `/api/${org_identifier}/roles/${role_id}`;
  return http().get(url);
};

export const deleteRole = (role_id: string, org_identifier: string) => {
  const url = `/api/${org_identifier}/roles/${role_id}`;
  return http().delete(url);
};

export const createRole = (role_id: string, org_identifier: string) => {
  const url = `/api/${org_identifier}/roles`;
  return http({}).post(url, {
    role: role_id,
  });
};

export const updateRole = (role: {
  role_id: string;
  org_identifier: string;
  payload: any;
}) => {
  const url = `/api/${role.org_identifier}/roles/${role.role_id}`;
  return http().put(url, role.payload);
};

export const getResources = (org_identifier: string) => {
  const url = `/api/${org_identifier}/resources`;
  return http().get(url);
};

export const getRoleUsers = (role_name: string, org_identifier: string) => {
  const url = `/api/${org_identifier}/roles/${role_name}/users`;
  return http().get(url);
};

// ----------- Permissions -------------

export const getResourcePermission = ({
  role_name,
  org_identifier,
  resource,
}: {
  role_name: string;
  org_identifier: string;
  resource: string;
}) => {
  const url = `/api/${org_identifier}/roles/${role_name}/permissions/${resource}`;
  return http().get(url);
};
