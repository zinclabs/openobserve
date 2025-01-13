// Copyright 2023 OpenObserve Inc.
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

import { reactive } from "vue";
import type { Resource } from "@/ts/interfaces";
import usersService from "@/services/users";
import service_accounts from "@/services/service_accounts";

const groups = {
  groups: {} as { [key: string]: any },
};

const roles = {
  roles: [],
};

const permissions = {
  permissions: [] as Resource[],
  selectedResources: {},
  resources: [] as any[],
};

const users = {
  users: [] as any[],
  getOrgUsers: (org_identifier: string, queryParams?: { list_all?: boolean } ) => {
    return new Promise((resolve, reject) => {
      usersService
        .orgUsers(org_identifier)
        .then((res) => {
          resolve(res.data.data);
        })
        .catch((err) => reject(err));
    });
  },
};
const service_accounts_users = {
  service_accounts_users: [] as any[],
  getServiceAccounts: (org_identifier: string) => {
    return new Promise((resolve, reject) => {
      service_accounts
        .list(org_identifier)
        .then((res) => {
          resolve(res.data.data);
        })
        .catch((err) => reject(err));
    });
  },
};

const permissionsState = reactive(Object.assign({}, permissions));

const rolesState = reactive(Object.assign({}, roles));

const groupsState = reactive(Object.assign({}, groups));

const usersState = reactive(Object.assign({}, users));

const serviceAccountsState = reactive(Object.assign({}, service_accounts_users));

const usePermissions = () => {
  const resetPermissionsState = () => {
    // delete searchObj.data;
    permissionsState.permissions = [];
    permissionsState.selectedResources = {};
  };

  const resetGroupsState = () => {};

  const resetRolesState = () => {};

  const resetUsersState = () => {};

  return {
    permissionsState,
    rolesState,
    groupsState,
    usersState,
    resetPermissionsState,
    resetGroupsState,
    resetRolesState,
    resetUsersState,
    serviceAccountsState,
  };
};

export default usePermissions;
