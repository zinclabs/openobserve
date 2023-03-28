import { DOMWrapper, mount } from "@vue/test-utils";
import { describe, expect, it, beforeEach } from "vitest";
import { installQuasar } from "../helpers/install-quasar-plugin";
import ConfirmDialog from "../../../components/ConfirmDialog.vue";
import i18n from "../../../locales";

installQuasar();

const node = document.createElement("div");
node.setAttribute("id", "app");
document.body.appendChild(node);

describe("ConfirmDialog", async () => {
  let wrapper: any = null;
  beforeEach(() => {
    // render the component
    wrapper = mount(ConfirmDialog, {
      attachTo: "#app",
      shallow: false,
      props: {
        title: "Dialog Title",
        message: "Dialog Message",
        confirmDelete: true,
      },
      global: {
        plugins: [i18n],
      },
    });
  });

  it("should mount ConfirmDialog component", async () => {
    const documentWrapper = new DOMWrapper(document.body);
    const dialog = documentWrapper.find(".q-dialog");
    expect(dialog.exists()).toBeFalsy();
  });
  it("should trigger cancel button", async () => {
    const documentWrapper = new DOMWrapper(document.body);
    const dialog = documentWrapper.find(".q-dialog");
    await dialog.get('[data-test="cancel-button"]').trigger("click");
  });
  it("should trigger confirm button", async () => {
    const documentWrapper = new DOMWrapper(document.body);
    const dialog = documentWrapper.find(".q-dialog");
    await dialog.get('[data-test="confirm-button"]').trigger("click");
  });
});
