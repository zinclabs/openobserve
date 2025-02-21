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

test.describe("dashboard tabs setting", () => {
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

  test("should try to open tabs, click add tabs, and without saving close it ", async ({
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
    await page.locator('[data-test="dashboard-settings-tab-tab"]').click();
    await page.locator('[data-test="dashboard-tab-settings-add-tab"]').click();
    await page.locator('[data-test="dashboard-add-cancel"]').click();
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
  });

  test("should go to tabs, click on add tab, add its name and save it", async ({
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
    await page.locator('[data-test="dashboard-settings-tab-tab"]').click();
    await page.locator('[data-test="dashboard-tab-settings-add-tab"]').click();
    await page.locator('[data-test="dashboard-add-tab-name"]').click();
    await page.locator('[data-test="dashboard-add-tab-name"]').fill("test");
    await page.locator('[data-test="dashboard-add-tab-submit"]').click();
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
    await expect(page.getByText("Dashboard added successfully.")).toBeVisible({
      timeout: 3000,
    });
    await expect(page.getByText("Tab added successfully")).toBeVisible({
      timeout: 2000,
    });
  });

  test("should edit tab name and save it", async ({ page }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-tab-tab"]').click();
    await page.locator('[data-test="dashboard-tab-settings-add-tab"]').click();
    await page.locator('[data-test="dashboard-add-tab-name"]').click();
    await page.locator('[data-test="dashboard-add-tab-name"]').fill("test");
    await page.locator('[data-test="dashboard-add-tab-submit"]').click();
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
    await expect(page.getByText("Dashboard added successfully.")).toBeVisible({
      timeout: 3000,
    });
    await expect(page.getByText("Tab added successfully")).toBeVisible({
      timeout: 2000,
    });
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-tab-tab"]').click();
    await page
      .locator('[data-test="dashboard-tab-settings-drag"] div')
      .filter({ hasText: "drag_indicatortestedit" })
      .locator('[data-test="dashboard-tab-settings-tab-edit-btn"]')
      .click();
    await page
      .locator('[data-test="dashboard-tab-settings-tab-name-edit"]')
      .click();
    await page
      .locator('[data-test="dashboard-tab-settings-tab-name-edit"]')
      .fill("test 2");
    await page
      .locator('[data-test="dashboard-tab-settings-tab-name-edit-save"]')
      .click();
    await expect(page.getByText("Tab updated successfully")).toBeVisible({
      timeout: 2000,
    });
  });

  test("should edit tab name and cancel it", async ({ page }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-tab-tab"]').click();
    await page.locator('[data-test="dashboard-tab-settings-add-tab"]').click();
    await page.locator('[data-test="dashboard-add-tab-name"]').click();
    await page.locator('[data-test="dashboard-add-tab-name"]').fill("test");
    await page.locator('[data-test="dashboard-add-tab-submit"]').click();
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
    await expect(page.getByText("Dashboard added successfully.")).toBeVisible({
      timeout: 3000,
    });
    await expect(page.getByText("Tab added successfully")).toBeVisible({
      timeout: 2000,
    });
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-tab-tab"]').click();
    await page
      .locator('[data-test="dashboard-tab-settings-drag"] div')
      .filter({ hasText: "drag_indicatortestedit" })
      .locator('[data-test="dashboard-tab-settings-tab-edit-btn"]')
      .click();
    await page
      .locator('[data-test="dashboard-tab-settings-tab-name-edit"]')
      .click();
    await page
      .locator('[data-test="dashboard-tab-settings-tab-name-edit"]')
      .fill("test 2");
    await page
      .locator('[data-test="dashboard-tab-settings-tab-name-edit-cancel"]')
      .click();
  });

  test("should delete tab, click delete and confirm it", async ({ page }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-tab-tab"]').click();
    await page.locator('[data-test="dashboard-tab-settings-add-tab"]').click();
    await page.locator('[data-test="dashboard-add-tab-name"]').click();
    await page.locator('[data-test="dashboard-add-tab-name"]').fill("test");
    await page.locator('[data-test="dashboard-add-tab-submit"]').click();
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
    await expect(page.getByText("Dashboard added successfully.")).toBeVisible({
      timeout: 3000,
    });
    await expect(page.getByText("Tab added successfully")).toBeVisible({
      timeout: 2000,
    });
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-settings-tab-tab"]').click();
    await page
      .locator('[data-test="dashboard-tab-settings-drag"] div')
      .filter({ hasText: "drag_indicatortestedit" })
      .locator('[data-test="dashboard-tab-settings-tab-delete-btn"]')
      .click();
    await page.locator('[data-test="dashboard-tab-delete-tab-para"]').click();
    await page.locator('[data-test="confirm-button"]').click();
    await expect(page.getByText("Tab deleted successfully")).toBeVisible({
      timeout: 2000,
    });
  });
});
