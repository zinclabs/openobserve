// Copyright 2022 Zinc Labs Inc. and Contributors

//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at

//      http:www.apache.org/licenses/LICENSE-2.0

//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

import { describe, expect, it, beforeEach, vi, afterEach } from "vitest";
import { mount, flushPromises, DOMWrapper } from "@vue/test-utils";
import { installQuasar } from "../../helpers/install-quasar-plugin";
import { Dialog, Notify } from "quasar";

import Index from "@/plugins/logs/Index.vue";
import i18n from "@/locales";
import store from "../../helpers/store";
import routes from "@/router/routes";
import { createRouter, createWebHistory } from "vue-router";
import { rest } from "msw";
import "plotly.js";
import logs from "../../mockData/logs";
import SearchResult from "@/plugins/logs/SearchResult.vue";
import searchService from "@/services/search";

const router = createRouter({
  history: createWebHistory(),
  routes,
});

const node = document.createElement("div");
node.setAttribute("id", "app");
document.body.appendChild(node);

installQuasar({
  plugins: [Dialog, Notify],
});

describe("Alert List", async () => {
  let wrapper: any;
  beforeEach(async () => {
    vi.useFakeTimers();
    wrapper = mount(Index, {
      attachTo: "#app",
      global: {
        provide: {
          store: store,
        },
        plugins: [i18n, router],
      },
    });
    await flushPromises();
  });

  afterEach(() => {
    wrapper.unmount();
    vi.restoreAllMocks();
    // vi.clearAllMocks();
  });

  it("Should render search bar when the stream list is not empty", async () => {
    expect(wrapper.get('[data-test="logs-search-bar"]').exists()).toBeTruthy();
  });

  it("Should hide search bar when the stream list is empty", async () => {
    vi.advanceTimersByTime(500);

    wrapper.vm.searchObj.data.stream.streamLists = [];
    await flushPromises();
    expect(
      wrapper.get('[data-test="logs-search-bar"]').attributes().style
    ).toContain("display: none;");
  });

  it("Should render index list when showFields is true.", async () => {
    vi.advanceTimersByTime(500);

    expect(
      wrapper.get('[data-test="logs-search-index-list"]').exists()
    ).toBeTruthy();
  });

  it("Should hide index list when showFields is false.", async () => {
    vi.advanceTimersByTime(500);

    wrapper.vm.searchObj.meta.showFields = false;
    await flushPromises();
    expect(
      wrapper.find('[data-test="logs-search-index-list"]').exists()
    ).toBeFalsy();
  });

  it("Should render search result component when there are query results", async () => {
    vi.advanceTimersByTime(500);

    expect(
      wrapper.get('[data-test="logs-search-search-result"]').exists()
    ).toBeTruthy();
  });

  it("Should hide search result component when there are no query results", async () => {
    // Set searchObj.data.queryResults.hits to an empty array.
    // Set searchObj.data.stream.selectedStream.label to a non-empty string.
    // Render the component.
    // Expect the search result component to not be displayed.
    global.server.use(
      rest.post(
        `${store.state.API_ENDPOINT}/api/${store.state.selectedOrganization.identifier}/_search`,
        (req, res, ctx) => {
          return res(
            ctx.status(200),
            ctx.json({
              took: 10,
              hits: [],
              total: 0,
              from: 0,
              size: 150,
              scan_size: 0,
            })
          );
        }
      )
    );

    vi.advanceTimersByTime(500);
    await flushPromises();
    expect(
      wrapper.get('[data-test="logs-search-search-result"]').attributes().style
    ).toContain("display: none;");
  });

  it("Should render 'No stream selected' message when no stream is selected", async () => {
    vi.advanceTimersByTime(500);

    // Set searchObj.data.stream.selectedStream.label to an empty string.
    // Render the component.
    // Expect the "No stream selected" message to be displayed.
    wrapper.vm.searchObj.data.stream.selectedStream.label = "";
    await flushPromises();
    expect(
      wrapper.find('[data-test="logs-search-no-stream-selected-text"]').text()
    ).toBe("No stream selected.");
  });

  it("Should render error message when error message is set", async () => {
    global.server.use(
      rest.post(
        `${store.state.API_ENDPOINT}/api/${store.state.selectedOrganization.identifier}/_search`,
        (req, res, ctx) => {
          return res(
            ctx.status(400),
            ctx.json({
              error: "Query Error",
              code: 400,
            })
          );
        }
      )
    );

    vi.advanceTimersByTime(500);
    // Set searchObj.data.errorMsg to a non-empty string.
    // Set searchObj.data.stream.selectedStream.label to a non-empty string.
    // Render the component.
    // Expect the error message to be displayed.
    await flushPromises();
    expect(wrapper.find('[data-test="logs-search-error-message"]').text()).toBe(
      "Query Error"
    );
  });

  it("Should render error when error code is 20003", async () => {
    await flushPromises();
    global.server.use(
      rest.post(
        `${store.state.API_ENDPOINT}/api/${store.state.selectedOrganization.identifier}/_search`,
        (req, res, ctx) => {
          return res(
            ctx.status(400),
            ctx.json({
              code: 20003,
            })
          );
        }
      )
    );

    vi.advanceTimersByTime(500);
    // Set searchObj.data.errorMsg to a non-empty string.
    // Set searchObj.data.stream.selectedStream.label to a non-empty string.
    // Render the component.
    // Expect the error message to be displayed.
    await flushPromises();
    expect(wrapper.find('[data-test="logs-search-error-20003"]').text()).toBe(
      "Click here to configure a full text search field to the stream."
    );
  });

  it("Should get logs data when scrolled in search results", async () => {
    const search = vi.spyOn(searchService, "search");
    wrapper.findComponent(SearchResult).vm.$emit("update:scroll");
    await vi.advanceTimersByTime(500);
    expect(search).toHaveBeenCalledTimes(1);
  });
});
