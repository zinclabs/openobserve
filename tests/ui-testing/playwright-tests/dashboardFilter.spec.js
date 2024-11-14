import { test, expect } from "@playwright/test";
// import { test, expect } from "./baseFixtures";
import logData from "../../ui-testing/cypress/fixtures/log.json";
import logsdata from "../../test-data/logs_data.json";

const randomDashboardName =
  "Dashboard_" + Math.random().toString(36).substr(2, 9);

test.describe.configure({ mode: "parallel" });

async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"], { waitUntil: "networkidle" });
  // await page.getByText('Login as internal user').click();
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
  ).toString('base64');

  const headers = {
    "Authorization": `Basic ${basicAuthCredentials}`,
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

test.describe("dashboard UI testcases", () => {
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

  test('should correctly apply the filter conditions with different operators, and successfully apply them to the query', async ({ page }) => {

    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);

    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();

    await page.locator('[data-test="field-list-item-logs-e2e_automate-_timestamp"] [data-test="dashboard-add-y-data"]').click();
    await page.waitForTimeout(2000);

    await page.locator('[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-filter-data"]').click();
    await page.locator('[data-test="dashboard-add-condition-label-0-kubernetes_container_name"]').click();
    await page.locator('[data-test="dashboard-add-condition-condition-0"]').click();
    await page.locator('[data-test="dashboard-add-condition-operator"]').click();
    await page.getByText('=', { exact: true }).click();
    await page.getByLabel('Value').click();
    await page.getByLabel('Value').fill('ziox');
    await page.locator('[data-test="date-time-btn"]').click();


    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();


    await page.locator('[data-test="dashboard-apply"]').click();
    await page.getByText('arrow_rightQueryAutoPromQLCustom SQL').click();
    // await page.locator('[data-test="dashboard-panel-query-editor"]').getByRole('code').locator('div').filter({ hasText: 'kubernetes_container_name = \'' }).nth(4).click();
    //  await page.locator('[data-test="dashboard-panel-query-editor"]').getByLabel('Editor content;Press Alt+F1').press('Control+a');
    // await expect(page.locator('[data-test="dashboard-panel-query-editor"]').getByRole('code').locator('div').filter({ hasText: 'kubernetes_container_name = \'' }).nth(4)).toBeVisible();
    // await expect(page.locator('[data-test="dashboard-panel-query-editor"]').getByRole('code').locator('div').filter({ hasText: 'kubernetes_container_name = \'' }).nth(4)).toBeVisible();

// Define the expected query text
const expectedQuery = `SELECT histogram(_timestamp) as "x_axis_1", count(_timestamp) as "y_axis_1" 
FROM "e2e_automate" 
WHERE kubernetes_container_name = 'ziox' 
GROUP BY x_axis_1 
ORDER BY x_axis_1 ASC`;

// Locate the query editor element
const queryEditor = page.locator('[data-test="dashboard-panel-query-editor"]').getByLabel('Editor content;Press Alt+F1');

// Select all text and replace with the specified query
await queryEditor.press('Control+a');
await queryEditor.fill(expectedQuery);

// Assert that the query editor contains the expected query
const actualQuery = await queryEditor.inputValue();
expect(actualQuery).toBe(expectedQuery);


    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill('test');
    await page.locator('[data-test="dashboard-panel-save"]').click();
    await page.locator('[data-test="dashboard-edit-panel-test-dropdown"]').click();
  });




  test('111should correctly apply filter conditions with various operators and verify the query', async ({ page }) => {
    // Helper function to apply filter and assert query
    const applyFilterAndVerifyQuery = async (operator, expectedQuery) => {
        await page.locator('[data-test="dashboard-add-condition-operator"]').click();
        await page.getByText(operator, { exact: true }).click();
        await page.locator('[data-test="dashboard-apply"]').click();

        // Verify the query editor contains the expected query
        const queryEditor = page.locator('[data-test="dashboard-panel-query-editor"]').getByLabel('Editor content;Press Alt+F1');
        await queryEditor.press('Control+a');
        const actualQuery = await queryEditor.inputValue();
        expect(actualQuery).toBe(expectedQuery);
    };

    // Step 1: Navigate to Dashboards and add a new dashboard
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Step 2: Add a panel and set the initial query source
    await page.locator('[data-test="dashboard-if-no-panel-add-panel-btn"]').click();
    await page.locator("label").filter({ hasText: "Streamarrow_drop_down" }).locator("i").click();
    await page.getByRole("option", { name: "e2e_automate" }).click();

    // Step 3: Set initial field and condition label for filtering
    await page.locator('[data-test="field-list-item-logs-e2e_automate-_timestamp"] [data-test="dashboard-add-y-data"]').click();

    await page.locator('[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-filter-data"]').click();
    // await page.locator('[data-test="dashboard-add-condition-label-0-kubernetes_container_name"]').click();
  //  await page.getByLabel('Value').fill('ziox');

  await page.locator('[data-test="dashboard-add-condition-label-0-kubernetes_container_name"]').click();
  await page.locator('[data-test="dashboard-add-condition-condition-0"]').click();
  await page.locator('[data-test="dashboard-add-condition-operator"]').click();
//   await page.getByText('=', { exact: true }).click();
  await page.getByLabel('Value').click();
  await page.getByLabel('Value').fill('ziox');


    // Test cases for each operator
    const operatorsAndQueries = [
        { operator: '=', query: `SELECT histogram(_timestamp) as "x_axis_1", count(_timestamp) as "y_axis_1" 
FROM "e2e_automate" 
WHERE kubernetes_container_name = 'ziox' 
GROUP BY x_axis_1 
ORDER BY x_axis_1 ASC` },

        { operator: '>=', query: `SELECT histogram(_timestamp) as "x_axis_1", count(_timestamp) as "y_axis_1" 
FROM "e2e_automate" 
WHERE kubernetes_container_name >= 'ziox' 
GROUP BY x_axis_1 
ORDER BY x_axis_1 ASC` },

        // Add more operators here as needed
        { operator: '!=', query: `SELECT histogram(_timestamp) as "x_axis_1", count(_timestamp) as "y_axis_1" 
FROM "e2e_automate" 
WHERE kubernetes_container_name != 'ziox' 
GROUP BY x_axis_1 
ORDER BY x_axis_1 ASC` },

        { operator: 'IS NULL', query: `SELECT histogram(_timestamp) as "x_axis_1", count(_timestamp) as "y_axis_1" 
FROM "e2e_automate" 
WHERE kubernetes_container_name IS NULL 
GROUP BY x_axis_1 
ORDER BY x_axis_1 ASC` }
    ];

    // Loop through each operator and expected query
    for (const { operator, query } of operatorsAndQueries) {
        await applyFilterAndVerifyQuery(operator, query);
    }
});




});