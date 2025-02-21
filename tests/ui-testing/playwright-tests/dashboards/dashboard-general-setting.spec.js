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

test.describe("dashboard general setting", () => {
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

  test("should verify that adding a default duration time automatically applies it as the default time on the chart", async ({
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
    await page.locator('[data-test="dashboard-general-setting-name"]').click();
    await page
      .locator('[data-test="dashboard-general-setting-description"]')
      .click();
    await page
      .locator('[data-test="dashboard-general-setting-description"]')
      .fill("test");
    await page
      .locator(
        '[data-test="dashboard-general-setting-datetime-picker"] [data-test="date-time-btn"]'
      )
      .click();
    await page.locator('[data-test="date-time-relative-2-h-btn"]').click();

    await page.waitForSelector(
      '[data-test="dashboard-general-setting-datetime-picker"] [data-test="date-time-btn"]'
    );
    const expectedTime = await page
      .locator(
        '[data-test="dashboard-general-setting-datetime-picker"] [data-test="date-time-btn"]'
      )
      .textContent();

    await page
      .locator('[data-test="dashboard-general-setting-save-btn"]')
      .click();
    await expect(page.getByText("Dashboard updated successfully")).toBeVisible({
      timeout: 30000,
    });
    await page.locator('[data-test="dashboard-settings-close-btn"]').click();
    await page.locator('[data-test="dashboard-panel-add"]').click();
    await page.locator('[data-test="dashboard-panel-discard"]').click();
    page.once("dialog", (dialog) => {
      dialog.dismiss().catch(() => {});
    });
    await page.waitForTimeout(2000);
    await page.locator('[data-test="date-time-btn"]').click();
    await page.waitForSelector('[data-test="date-time-btn"]'); // Wait for panel time display
    const panelTime = await page
      .locator('[data-test="date-time-btn"]')
      .textContent();
    expect(panelTime).toBe(expectedTime);
  });

  test("should verify that dynamic toggle is diabled", async ({ page }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.locator('[data-test="dashboard-general-setting-name"]').click();
    await page
      .locator('[data-test="dashboard-general-setting-description"]')
      .click();
    await page
      .locator('[data-test="dashboard-general-setting-description"]')
      .fill("test");
    await page
      .locator('[data-test="dashboard-general-setting-dynamic-filter"] div')
      .nth(2)
      .click();
    await page
      .locator('[data-test="dashboard-general-setting-save-btn"]')
      .click();
    await expect(page.getByText("Dashboard updated successfully")).toBeVisible({
      timeout: 30000,
    });
  });
});
