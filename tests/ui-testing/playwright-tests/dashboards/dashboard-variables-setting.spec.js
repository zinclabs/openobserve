import { test, expect } from "../baseFixtures";
import logData from "../../cypress/fixtures/log.json";
import logsdata from "../../../test-data/logs_data.json";

const randomDashboardName =
  "Dashboard_" + Math.random().toString(36).slice(2, 11);

test.describe.configure({ mode: "parallel" });

async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"], { waitUntil: "networkidle" });

  if (await page.getByText("Login as internal user").isVisible()) {
    await page.getByText("Login as internal user").click();
  }

  await page.waitForTimeout(1000);
  await page
    .locator('[data-cy="login-user-id"]')
    .fill(process.env["ZO_ROOT_USER_EMAIL"]);

  // wait for login api response
  const waitForLogin = page.waitForResponse(
    (response) =>
      response.url().includes("/auth/login") && response.status() === 200
  );

  await page
    .locator('[data-cy="login-password"]')
    .fill(process.env["ZO_ROOT_USER_PASSWORD"]);
  await page.locator('[data-cy="login-sign-in"]').click();

  await waitForLogin;

  await page.waitForURL(process.env["ZO_BASE_URL"] + "/web/", {
    waitUntil: "networkidle",
  });
  await page
    .locator('[data-test="navbar-organizations-select"]')
    .getByText("arrow_drop_down")
    .click();
  await page.getByRole("option", { name: "default", exact: true }).click();
}

async function ingestion(page) {
  const orgId = process.env["ORGNAME"];
  const streamName = "e2e_automate";
  const basicAuthCredentials = Buffer.from(
    `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
  ).toString("base64");

  const headers = {
    Authorization: `Basic ${basicAuthCredentials}`,
    "Content-Type": "application/json",
  };
  const fetchResponse = await fetch(
    `${process.env.INGESTION_URL}/api/${orgId}/${streamName}/_json`,
    {
      method: "POST",
      headers: headers,
      body: JSON.stringify(logsdata),
    }
  );
  const response = await fetchResponse.json();
  console.log(response);
}

async function waitForDashboardPage(page) {
  const dashboardListApi = page.waitForResponse(
    (response) =>
      /\/api\/.+\/dashboards/.test(response.url()) && response.status() === 200
  );

  await page.waitForURL(process.env["ZO_BASE_URL"] + "/web/dashboards**");

  await page.waitForSelector(`text="Please wait while loading dashboards..."`, {
    state: "hidden",
  });
  await dashboardListApi;
  await page.waitForTimeout(500);
}

test.describe("dashboard variables setting", () => {
  // let logData;
  function removeUTFCharacters(text) {
    // console.log(text, "tex");
    // Remove UTF characters using regular expression
    return text.replace(/[^\x00-\x7F]/g, " ");
  }
  async function applyQueryButton(page) {
    // click on the run query button
    // Type the value of a variable into an input field
    const search = page.waitForResponse(logData.applyQuery);
    await page.waitForTimeout(3000);
    await page.locator("[data-test='logs-search-bar-refresh-btn']").click({
      force: true,
    });
    // get the data from the search variable
    await expect.poll(async () => (await search).status()).toBe(200);
    // await search.hits.FIXME_should("be.an", "array");
  }
  // tebefore(async function () {
  //   // logData("log");
  //   // const data = page;
  //   // logData = data;

  //   console.log("--logData--", logData);
  // });
  test.beforeEach(async ({ page }) => {
    console.log("running before each");
    await login(page);
    await page.waitForTimeout(1000);
    await ingestion(page);
    await page.waitForTimeout(2000);

    // just to make sure org is set
    const orgNavigation = page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );

    await orgNavigation;
  });

  test("should try to open variables, click add variable, and without saving close it ", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-cancel-btn"]').click();
  });

  // query values test cases
  test("should add query_values to dashboard and save it", async ({ page }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-type-select"]').click();
    await page
      .getByRole("option", { name: "Query Values" })
      .locator("span")
      .click();
    await page.locator('[data-test="dashboard-variable-name"]').click();
    await page
      .locator('[data-test="dashboard-variable-name"]')
      .fill("query_value_test");
    await page.locator('[data-test="dashboard-variable-label"]').click();
    await page
      .locator('[data-test="dashboard-variable-label"]')
      .fill("namespace");
    await page
      .locator('[data-test="dashboard-variable-stream-type-select"]')
      .click();
    await page
      .getByRole("option", { name: "logs" })
      .locator("div")
      .nth(2)
      .click();
    await page
      .locator('[data-test="dashboard-variable-stream-select"]')
      .click();
    await page
      .getByRole("option", { name: "e2e_automate" })
      .locator("span")
      .click();
    await page.locator('[data-test="dashboard-variable-field-select"]').click();
    await page
      .locator('[data-test="dashboard-variable-field-select"]')
      .fill("names");
    await page.getByText("kubernetes_namespace_name").click();
    await page.locator('[data-test="dashboard-variable-save-btn"]').click();
    await page.waitForTimeout(2000);
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
    await page
      .locator('[data-test="dashboard-variable-query-value-selector"]')
      .click();
  });

  test("should add max records size, check toggle of multi select, and save it", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-type-select"]').click();
    await page
      .getByRole("option", { name: "Query Values" })
      .locator("span")
      .click();
    await page.locator('[data-test="dashboard-variable-name"]').click();
    await page
      .locator('[data-test="dashboard-variable-name"]')
      .fill("query_value_test");
    await page.locator('[data-test="dashboard-variable-label"]').click();
    await page
      .locator('[data-test="dashboard-variable-label"]')
      .fill("namespace");
    await page
      .locator('[data-test="dashboard-variable-stream-type-select"]')
      .click();
    await page
      .getByRole("option", { name: "logs" })
      .locator("div")
      .nth(2)
      .click();
    await page
      .locator('[data-test="dashboard-variable-stream-select"]')
      .click();
    await page
      .getByRole("option", { name: "e2e_automate" })
      .locator("span")
      .click();
    await page.locator('[data-test="dashboard-variable-field-select"]').click();
    await page
      .locator('[data-test="dashboard-variable-field-select"]')
      .fill("names");
    await page.getByText("kubernetes_namespace_name").click();
    await page
      .locator('[data-test="dashboard-variable-max-record-size"]')
      .click();
    await page
      .locator('[data-test="dashboard-variable-max-record-size"]')
      .fill("2");
    await page
      .locator('[data-test="dashboard-query_values-show_multiple_values"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-variable-save-btn"]').click();
    await page.waitForTimeout(2000);
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
    await page
      .locator('[data-test="dashboard-variable-query-value-selector"]')
      .click();
  });

  test("should verify that by default select is working", async ({ page }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-type-select"]').click();
    await page
      .getByRole("option", { name: "Query Values" })
      .locator("span")
      .click();
    await page.locator('[data-test="dashboard-variable-name"]').click();
    await page
      .locator('[data-test="dashboard-variable-name"]')
      .fill("query_value_test");
    await page.locator('[data-test="dashboard-variable-label"]').click();
    await page
      .locator('[data-test="dashboard-variable-label"]')
      .fill("namespace");
    await page
      .locator('[data-test="dashboard-variable-stream-type-select"]')
      .click();
    await page
      .getByRole("option", { name: "logs" })
      .locator("div")
      .nth(2)
      .click();
    await page
      .locator('[data-test="dashboard-variable-stream-select"]')
      .click();
    await page
      .getByRole("option", { name: "e2e_automate" })
      .locator("span")
      .click();
    await page.locator('[data-test="dashboard-variable-field-select"]').click();
    await page
      .locator('[data-test="dashboard-variable-field-select"]')
      .fill("names");
    await page.getByText("kubernetes_namespace_name").click();
    await page
      .locator('[data-test="dashboard-variable-max-record-size"]')
      .click();
    await page
      .locator('[data-test="dashboard-variable-max-record-size"]')
      .fill("2");
    await page
      .locator('[data-test="dashboard-query_values-show_multiple_values"] div')
      .nth(2)
      .click();
    await page
      .locator(
        '[data-test="dashboard-multi-select-default-value-toggle-first-value"]'
      )
      .click();
    await page
      .locator(
        '[data-test="dashboard-multi-select-default-value-toggle-all-values"]'
      )
      .click();
    await page
      .locator(
        '[data-test="dashboard-multi-select-default-value-toggle-custom"]'
      )
      .click();
    await page.locator('[data-test="dashboard-add-custom-value-btn"]').click();
    await page
      .locator('[data-test="dashboard-variable-custom-value-0"]')
      .click();
    await page
      .locator('[data-test="dashboard-variable-custom-value-0"]')
      .fill("ingress-nginx");
    await page.locator('[data-test="dashboard-variable-save-btn"]').click();
    await page.waitForTimeout(2000);
    await page.locator('[data-test="dashboard-variable-query-value-selector"]');
    await expect(page.getByText("ingress-nginx")).toBeVisible({
      timeout: 30000,
    });
  });

  test("should verify that hide on query_values variable dashboard is working", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-type-select"]').click();
    await page
      .getByRole("option", { name: "Query Values" })
      .locator("span")
      .click();
    await page.locator('[data-test="dashboard-variable-name"]').click();
    await page
      .locator('[data-test="dashboard-variable-name"]')
      .fill("query_value_test");
    await page.locator('[data-test="dashboard-variable-label"]').click();
    await page
      .locator('[data-test="dashboard-variable-label"]')
      .fill("namespace");
    await page
      .locator('[data-test="dashboard-variable-stream-type-select"]')
      .click();
    await page
      .getByRole("option", { name: "logs" })
      .locator("div")
      .nth(2)
      .click();
    await page
      .locator('[data-test="dashboard-variable-stream-select"]')
      .click();
    await page
      .getByRole("option", { name: "e2e_automate" })
      .locator("span")
      .click();
    await page.locator('[data-test="dashboard-variable-field-select"]').click();
    await page
      .locator('[data-test="dashboard-variable-field-select"]')
      .fill("names");
    await page.getByText("kubernetes_namespace_name").click();
    await page
      .locator('[data-test="dashboard-variable-hide_on_dashboard"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-variable-save-btn"]').click();
  });

  // constant test cases
  test("should verify constant variable by adding and verify that its visible on dashboard", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-type-select"]').click();
    await page.getByRole("option", { name: "Constant" }).click();
    await page.locator('[data-test="dashboard-variable-name"]').click();
    await page
      .locator('[data-test="dashboard-variable-name"]')
      .fill("constant");
    await page.locator('[data-test="dashboard-variable-label"]').click();
    await page
      .locator('[data-test="dashboard-variable-label"]')
      .fill("constant-variable");
    await page
      .locator('[data-test="dashboard-variable-constant-value"]')
      .click();
    await page
      .locator('[data-test="dashboard-variable-constant-value"]')
      .fill("200");
    await page.locator('[data-test="dashboard-variable-save-btn"]').click();
    await page.waitForTimeout(2000);
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
    await page
      .locator('[data-test="dashboard-variable-constant-selector"]')
      .click();
  });

  test("should verify that hide on constant variable dashboard is working", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-type-select"]').click();
    await page.getByRole("option", { name: "Constant" }).click();
    await page.locator('[data-test="dashboard-variable-name"]').click();
    await page
      .locator('[data-test="dashboard-variable-name"]')
      .fill("constant");
    await page.locator('[data-test="dashboard-variable-label"]').click();
    await page
      .locator('[data-test="dashboard-variable-label"]')
      .fill("constant-variable");
    await page
      .locator('[data-test="dashboard-variable-constant-value"]')
      .click();
    await page
      .locator('[data-test="dashboard-variable-constant-value"]')
      .fill("200");
    await page
      .locator('[data-test="dashboard-variable-hide_on_dashboard"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-variable-save-btn"]').click();
  });

  // textbox test cases
  test("should verify textbox variable by adding and verify that its visible on dashboard", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-type-select"]').click();
    await page.getByRole("option", { name: "Textbox" }).click();
    await page.locator('[data-test="dashboard-variable-name"]').click();
    await page.locator('[data-test="dashboard-variable-name"]').fill("textbox");
    await page.locator('[data-test="dashboard-variable-label"]').click();
    await page
      .locator('[data-test="dashboard-variable-label"]')
      .fill("textbox-variable");
    await page
      .locator('[data-test="dashboard-variable-textbox-default-value"]')
      .click();
    await page
      .locator('[data-test="dashboard-variable-textbox-default-value"]')
      .fill("500");
    await page.locator('[data-test="dashboard-variable-save-btn"]').click();
    await page.waitForTimeout(2000);
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
    await page
      .locator('[data-test="dashboard-variable-textbox-selector"]')
      .click();
    await page
      .locator('[data-test="dashboard-variable-textbox-selector"]')
      .fill("800");
    await expect(
      page.locator('[data-test="dashboard-variable-textbox-selector"]')
    ).toHaveValue("800");
  });

  test("should verify that hide on textbox variable dashboard is working", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-type-select"]').click();
    await page.getByRole("option", { name: "Textbox" }).click();
    await page.locator('[data-test="dashboard-variable-name"]').click();
    await page.locator('[data-test="dashboard-variable-name"]').fill("textbox");
    await page.locator('[data-test="dashboard-variable-label"]').click();
    await page
      .locator('[data-test="dashboard-variable-label"]')
      .fill("textbox-variable");
    await page
      .locator('[data-test="dashboard-variable-textbox-default-value"]')
      .click();
    await page
      .locator('[data-test="dashboard-variable-textbox-default-value"]')
      .fill("500");
    await page
      .locator('[data-test="dashboard-variable-hide_on_dashboard"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-variable-save-btn"]').click();
  });

  // custom test cases
  test("should verify custom variables by adding and its visible on dashboard", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-type-select"]').click();
    await page
      .getByRole("option", { name: "Custom" })
      .locator("div")
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-variable-name"]').click();
    await page.locator('[data-test="dashboard-variable-name"]').fill("custom");
    await page.locator('[data-test="dashboard-variable-label"]').click();
    await page
      .locator('[data-test="dashboard-variable-label"]')
      .fill("custom-variable");
    await page.getByRole("button", { name: "Add Option" }).click();
    await page
      .locator('[data-test="dashboard-custom-variable-0-label"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-0-label"]')
      .fill("v1");
    await page
      .locator('[data-test="dashboard-custom-variable-0-value"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-0-value"]')
      .fill("v1");
    await page.getByRole("button", { name: "Add Option" }).click();
    await page
      .locator('[data-test="dashboard-custom-variable-1-label"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-1-label"]')
      .fill("v2");
    await page
      .locator('[data-test="dashboard-custom-variable-1-value"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-1-value"]')
      .fill("v2");
    await page
      .locator('[data-test="dashboard-custom-variable-0-checkbox"]')
      .click();
    await page.locator('[data-test="dashboard-variable-save-btn"]').click();
    await page.waitForTimeout(2000);
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
    await page
      .locator('[data-test="dashboard-variable-custom-value-selector"]')
      .click();
  });

  test("should verify that multi select is working properly", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-type-select"]').click();
    await page
      .getByRole("option", { name: "Custom" })
      .locator("div")
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-variable-name"]').click();
    await page.locator('[data-test="dashboard-variable-name"]').fill("custom");
    await page.locator('[data-test="dashboard-variable-label"]').click();
    await page
      .locator('[data-test="dashboard-variable-label"]')
      .fill("custom-variable");
    await page.getByRole("button", { name: "Add Option" }).click();
    await page
      .locator('[data-test="dashboard-custom-variable-0-label"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-0-label"]')
      .fill("v1");
    await page
      .locator('[data-test="dashboard-custom-variable-0-value"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-0-value"]')
      .fill("v1");
    await page.getByRole("button", { name: "Add Option" }).click();
    await page
      .locator('[data-test="dashboard-custom-variable-1-label"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-1-label"]')
      .fill("v2");
    await page
      .locator('[data-test="dashboard-custom-variable-1-value"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-1-value"]')
      .fill("v2");
    await page
      .locator('[data-test="dashboard-query_values-show_multiple_values"] div')
      .nth(2)
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-0-checkbox"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-1-checkbox"]')
      .click();
    await page.locator('[data-test="dashboard-variable-save-btn"]').click();
    await page.waitForTimeout(2000);
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
    await page
      .locator('[data-test="dashboard-variable-custom-value-selector"]')
      .click();
  });

  test("should verify that hide on custom variable dashboard is working", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
    await page.locator('[data-test="dashboard-variable-add-btn"]').click();
    await page.locator('[data-test="dashboard-variable-type-select"]').click();
    await page
      .getByRole("option", { name: "Custom" })
      .locator("div")
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-variable-name"]').click();
    await page.locator('[data-test="dashboard-variable-name"]').fill("custom");
    await page.locator('[data-test="dashboard-variable-label"]').click();
    await page
      .locator('[data-test="dashboard-variable-label"]')
      .fill("custom-variable");
    await page.getByRole("button", { name: "Add Option" }).click();
    await page
      .locator('[data-test="dashboard-custom-variable-0-label"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-0-label"]')
      .fill("v1");
    await page
      .locator('[data-test="dashboard-custom-variable-0-value"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-0-value"]')
      .fill("v1");
    await page.getByRole("button", { name: "Add Option" }).click();
    await page
      .locator('[data-test="dashboard-custom-variable-1-label"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-1-label"]')
      .fill("v2");
    await page
      .locator('[data-test="dashboard-custom-variable-1-value"]')
      .click();
    await page
      .locator('[data-test="dashboard-custom-variable-1-value"]')
      .fill("v2");
    await page
      .locator('[data-test="dashboard-variable-hide_on_dashboard"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-variable-save-btn"]').click();
  });

  // test("variable value should be displayed when applying the dynamic filter and refreshing the page", async ({
  //   page,
  // }) => {
  //   await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
  //   await waitForDashboardPage(page);
  //   await page.locator('[data-test="dashboard-add"]').click();
  //   await page.locator('[data-test="add-dashboard-name"]').click();
  //   await page
  //     .locator('[data-test="add-dashboard-name"]')
  //     .fill(randomDashboardName);

  //   await page.locator('[data-test="dashboard-add-submit"]').click();
  //   await page.waitForTimeout(3000);

  //   await page.locator('[data-test="dashboard-setting-btn"]').click();

  //   await page.locator('[data-test="dashboard-settings-variable-tab"]').click();
  //   await page.locator('[data-test="dashboard-variable-add-btn"]').click();

  //   await page.locator('[data-test="dashboard-variable-name"]').fill('variablename');

  //   await page
  //     .locator("label")
  //     .filter({ hasText: "Stream Type *arrow_drop_down" })
  //     .locator("i")
  //     .click();
  //     await page.getByRole('option', { name: 'logs' }).locator('div').nth(2).click();

  //   await page
  //     .locator('[data-test="dashboard-variable-stream-select"]')
  //     .fill("e2e_automate");
  //   await page
  //     .getByRole("option", { name: "e2e_automate", exact: true })
  //     .locator("div")
  //     .nth(2)
  //     .click();

  //     await page.locator('[data-test="dashboard-variable-field-select"]').click();
  //   await page.locator('[data-test="dashboard-variable-field-select"]').fill('kubernetes_container_name');
  //   await page.getByText('kubernetes_container_name').click();

  //   await page.locator('[data-test="dashboard-variable-save-btn"]').click();

  //   await page.waitForTimeout(3000);
  //   await page.locator('[data-test="dashboard-settings-close-btn"]').click();

  //   const button = page.locator(
  //     '[data-test="dashboard-if-no-panel-add-panel-btn"]'
  //   );
  //   await expect(button).toBeVisible();

  //   await page.waitForTimeout(3000);
  //   await button.click();

  //   await page.waitForTimeout(2000);

  //   await page
  //     .locator('[data-test="index-dropdown-stream"]')
  //     .waitFor({ state: "visible" });
  //   await page.locator('[data-test="index-dropdown-stream"]').click();

  //   await page.waitForTimeout(2000);

  //   await page
  //     .getByRole("option", { name: "e2e_automate", exact: true })
  //     .click();

  //   await page
  //     .locator(
  //       '[data-test="field-list-item-logs-e2e_automate-_timestamp"] [data-test="dashboard-add-y-data"]'
  //     )
  //     .click();

  //   await page
  //     .locator(
  //       '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-b-data"]'
  //     )
  //     .click();

  //   await page.locator('[data-test="dashboard-apply"]').click();

  //   await page.locator('[data-test="date-time-btn"]').click();
  //   await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
  //   await page.locator('[data-test="date-time-apply-btn"]').click();
  //   await page.locator('[data-test="dashboard-panel-name"]').click();
  //   await page.locator('[data-test="dashboard-panel-name"]').click();
  //   await page.locator('[data-test="dashboard-panel-name"]').fill("test");
  //   await page.locator('[data-test="dashboard-panel-save"]').click();

  //   await page.waitForTimeout(3000);

  //   await page.locator('[data-test="date-time-btn"]').click();
  //   await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
  //   await page.locator('[data-test="date-time-apply-btn"]').click();

  //   await page
  //     .locator('[data-test="dashboard-variable-adhoc-add-selector"]')
  //     .click();
  //   await page
  //     .locator('[data-test="dashboard-variable-adhoc-name-selector"]')
  //     .click();
  //   await page
  //     .locator('[data-test="dashboard-variable-adhoc-name-selector"]')
  //     .fill("kubernetes_container_name");
  //   await page
  //     .locator('[data-test="dashboard-variable-adhoc-value-selector"]')
  //     .click();
  //   await page
  //     .locator('[data-test="dashboard-variable-adhoc-value-selector"]')
  //     .fill("ziox");

  //   await page.locator('[data-test="dashboard-refresh-btn"]').click();
  //   await page.reload();

  //   // Click on the dropdown
  //   await page.getByText("controllervariablenamearrow_drop_down").click();

  //   // Check if the dropdown contains options
  //   const dropdownOptions = page.getByRole("option"); // Don't await yet, just capture the locator
  //   const dropdownCount = await dropdownOptions.count();

  //   console.log("Dropdown count:", dropdownCount); // Debugging line

  //   // Ensure the dropdown options are not blank
  //   expect(dropdownCount).toBeGreaterThan(0); // Checks that there is at least one option

});
