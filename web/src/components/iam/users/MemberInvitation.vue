<!-- Copyright 2023 OpenObserve Inc.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
-->

<template>
  <q-page class="q-pa-none" style="min-height: inherit">
    <div
      class="col-12 flex"
      v-if="currentUserRole == 'admin' || currentUserRole == 'root'"
    >
      <q-separator vertical class="separator q-mr-sm" />

      <div
        class="row invite-user"
        style="width: calc(100% - 110px); display: inline-flex"
      >
        <q-input
          v-model="userEmail"
          borderless
          filled
          dense
          :placeholder="t('user.inviteByEmail')"
          style="width: calc(100% - 120px)"
          class="q-pr-sm"
        />
        <div class="flex justify-center">
          <q-select
            dense
            filled
            borderless
            v-model="selectedRole"
            option-label="label"
            option-value="value"
            emit-value
            map-options
            :options="options"
            style="width: 120px"
            class="q-pr-sm"
          />
        </div>
      </div>
      <q-btn
        class="text-bold no-border"
        padding="sm 0"
        color="secondary"
        no-caps
        :label="t(`user.sendInvite`)"
        @click="inviteUser()"
        :disable="userEmail == ''"
        style="
          padding: 7px 9px;
          min-height: 0px;
          width: 100px;
          display: block;
          float: right;
          top: 1px;
        "
      />
      <label class="inputHint q-pl-md" style="display: block">{{
        t("user.inviteByEmailHint")
      }}</label>
    </div>
  </q-page>
</template>

<script lang="ts">
import { defineComponent, ref, onBeforeMount } from "vue";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import { useI18n } from "vue-i18n";
import { validateEmail } from "@/utils/zincutils";
import organizationsService from "@/services/organizations";
import segment from "@/services/segment_analytics";
import { getRoles } from "@/services/iam";
import usersService from "@/services/users";

export default defineComponent({
  name: "MemberInvitationPage",
  props: {
    currentrole: {
      type: String,
      required: true,
    },
  },
  setup(props: any) {
    const store = useStore();
    const { t } = useI18n();
    const $q = useQuasar();

    const userEmail: any = ref("");
    const options = ref();
    const selectedRole = ref("admin");
    const currentUserRole = ref(props.currentrole || "");

    onBeforeMount(async () => {
      currentUserRole.value = props.currentrole;
      await getRoles();
    });

    const inviteUser = () => {
      const emailArray = userEmail.value
        .split(";")
        .flatMap((email: any) => email.split(","))
        .filter((email: any) => email.trim().length > 0)
        .map((email: any) => email.trim().toLowerCase());
      const validationArray = emailArray.map((email: any) =>
        validateEmail(email),
      );

      if (!validationArray.includes(false)) {
        const dismiss = $q.notify({
          spinner: true,
          message: "Please wait...",
          timeout: 2000,
        });

        organizationsService
          .add_members(
            { invites: emailArray, role: selectedRole.value },
            store.state.selectedOrganization.identifier,
          )
          .then((res: { data: any }) => {
            const data = res.data;

            if (data.data.invalid_members != null) {
              const message = `Error while member invitation: ${data.data.invalid_members.toString()}`;

              $q.notify({
                type: "negative",
                message: message,
                timeout: 15000,
              });
            } else {
              $q.notify({
                type: "positive",
                message: data.message,
                timeout: 5000,
              });
            }

            dismiss();
          })
          .catch((error) => {
            dismiss();
            $q.notify({
              type: "negative",
              message: error.message,
              timeout: 5000,
            });
          });

        userEmail.value = "";

        segment.track("Button Click", {
          button: "Invite User",
          user_org: store.state.selectedOrganization.identifier,
          user_id: store.state.userInfo.email,
          page: "Users",
        });
      } else {
        $q.notify({
          type: "negative",
          message: `Please enter correct email id.`,
        });
      }
    };

    const getRoles = () => {
      return new Promise((resolve) => {
        usersService
          .getRoles(store.state.selectedOrganization.identifier)
          .then((res) => {
            options.value = res.data;
          })
          .finally(() => resolve(true));
      });
    };

    return {
      t,
      userEmail,
      selectedRole,
      options,
      inviteUser,
      currentUserRole,
    };
  },
});
</script>

<style lang="scss" scoped>
.invite-user {
  background: $input-bg;
  border-radius: 4px;

  .separator {
    width: 1px;
  }
}

.inputHint {
  font-size: 11px;
  color: $light-text;
}
</style>
