import { test, expect } from "../baseFixtures";
import logData from "../../cypress/fixtures/log.json";
import logsdata from "../../../test-data/logs_data.json";

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

  test("should correctly apply the filter conditions with different operators, and successfully apply them to the query", async ({
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
    await page.getByRole("tab", { name: "Variables" }).click();
    await page.getByRole("button", { name: "Add Variable" }).click();
    await page
      .locator("div")
      .filter({ hasText: /^Name \*$/ })
      .nth(2)
      .click();

    // await page.getByLabel('Name *').click();
    await page.getByLabel("Name *").fill("variablename");

    await page
      .locator("label")
      .filter({ hasText: "Stream Type *arrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "logs" }).click();
    await page
      .locator("label")
      .filter({ hasText: "Stream *arrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();
    await page.getByText("Field *arrow_drop_down").click();
    await page
      .getByRole("option", { name: "kubernetes_container_name" })
      .locator("div")
      .nth(2)
      .click();
    await page.getByRole("button", { name: "Save" }).click();

    await page.waitForTimeout(3000);

    const button = page.locator(
      '[data-test="dashboard-if-no-panel-add-panel-btn"]'
    );
    await expect(button).toBeVisible();

    await page.waitForTimeout(3000);
    await button.click();

    // await page
    //   .locator("label")
    //   .filter({ hasText: "Streamarrow_drop_down" })
    //   .locator("i")
    //   .click();

      await page.waitForTimeout(2000);

    await page.locator('[data-test="index-dropdown-stream"]').waitFor({ state: 'visible' });
    await page.locator('[data-test="index-dropdown-stream"]').click();

    // await page.getByText('Streamarrow_drop_down').click();
//     const streamDropdown = page.locator('[data-test="index-dropdown-stream"]').click();
// await expect(streamDropdown).toBeVisible();
// await streamDropdown.click();

     await page.waitForTimeout(2000);

    await page.getByRole("option", { name: "e2e_automate", exact: true }).click();

    // await page.waitForTimeout(2000);

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-_timestamp"] [data-test="dashboard-add-y-data"]'
      )
      .click();

      await page.locator('[data-test="dashboard-apply"]').click();

    await page.waitForTimeout(3000);


    // await page
    //   .locator(
    //     '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-filter-data"]'
    //   )
    //   .click();
    const filterButton = page.locator('[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-filter-data"]');
await expect(filterButton).toBeVisible();
await filterButton.click();


    await page
      .locator('[data-test="dashboard-variable-query-value-selector"]')
      .click();
    await page.getByRole("option", { name: "ziox" }).click();

    await page.locator('[data-test="dashboard-add-condition-add"]').click();

    await page
      .locator(
        '[data-test="dashboard-add-condition-label-0-kubernetes_container_name"]'
      )
      .click();
    await page
      .locator('[data-test="dashboard-add-condition-condition-0"]')
      .click();
    await page.getByText("Operatorarrow_drop_down").click();
    await page.getByText("=", { exact: true }).click();
    await page.getByLabel("Value").click();
    await page.getByLabel('Value').fill('$variablename');

    // await page
    //   .locator("#q-portal--menu--183")
    //   .getByText("variablename")
    //   .click();
      // await page.getByRole("option", { name: "variablename", exact: true }).click();

    await page.locator('[data-test="dashboard-apply"]').click();

    await page.getByText("arrow_rightQueryAutoPromQLCustom SQL").click();
    await expect(page.getByText("'$variablename'")).toBeVisible();

    // Click the 'variablename' option in the dropdown menu

    // const variableName = await page
    //   .locator("#q-portal--menu--183")
    //   .getByText("variablename");
    // await variableName.click();

    await page.locator('[data-test="dashboard-apply"]').click();

    // Perform the action that applies the variable (e.g., clicking "QueryAutoPromQLCustom SQL")
    // await page.getByText("arrow_rightQueryAutoPromQLCustom SQL").click();
    await page.waitForTimeout(2000);

    await page.locator('[data-test="dashboard-panel-data-view-query-inspector-btn"]').click();
  await expect(page.getByRole('cell', { name: 'SELECT histogram(_timestamp) as "x_axis_1", count(_timestamp) as "y_axis_1" FROM "e2e_automate" WHERE kubernetes_container_name = \'ziox\' GROUP BY x_axis_1 ORDER BY x_axis_1 ASC', exact: true })).toBeVisible();
   
  
    await page.locator('[data-test="query-inspector-close-btn"]').click();

    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("test");
    await page.locator('[data-test="dashboard-panel-save"]').click();
  });

  test("should successfully apply filter conditions using both AND and OR operators", async ({
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
    await page.getByRole("tab", { name: "Variables" }).click();
    await page.getByRole("button", { name: "Add Variable" }).click();
    await page
      .locator("div")
      .filter({ hasText: /^Name \*$/ })
      .nth(2)
      .click();

    // await page.getByLabel('Name *').click();
    await page.getByLabel("Name *").fill("variablename");

    await page
      .locator("label")
      .filter({ hasText: "Stream Type *arrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "logs" }).click();
    await page
      .locator("label")
      .filter({ hasText: "Stream *arrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();
    await page.getByText("Field *arrow_drop_down").click();
    await page
      .getByRole("option", { name: "kubernetes_container_name" })
      .locator("div")
      .nth(2)
      .click();
    await page.getByRole("button", { name: "Save" }).click();

    await page.waitForTimeout(3000);

    const button = page.locator(
      '[data-test="dashboard-if-no-panel-add-panel-btn"]'
    );
    await expect(button).toBeVisible();

    await page.waitForTimeout(3000);
    await button.click();

    // await page
    //   .locator("label")
    //   .filter({ hasText: "Streamarrow_drop_down" })
    //   .locator("i")
    //   .click();

      await page.waitForTimeout(2000);

    await page.locator('[data-test="index-dropdown-stream"]').waitFor({ state: 'visible' });
    await page.locator('[data-test="index-dropdown-stream"]').click();

    // await page.getByText('Streamarrow_drop_down').click();
//     const streamDropdown = page.locator('[data-test="index-dropdown-stream"]').click();
// await expect(streamDropdown).toBeVisible();
// await streamDropdown.click();

     await page.waitForTimeout(2000);

    await page.getByRole("option", { name: "e2e_automate", exact: true }).click();

    // await page.waitForTimeout(2000);

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-_timestamp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
     await page.locator('[data-test="dashboard-apply"]').click();

     await page.locator('[data-test="date-time-btn"]').click();
     await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
     await page.locator('[data-test="date-time-apply-btn"]').click();

    await page.waitForTimeout(3000);


    // await page
    //   .locator(
    //     '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-filter-data"]'
    //   )
    //   .click();
    const filterButton = page.locator('[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-filter-data"]');
await expect(filterButton).toBeVisible();
await filterButton.click();

    // await page
    //   .locator(
    //     '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-filter-data"]'
    //   )
    //   .click();

    const filterButton1 = page.locator('[data-test="field-list-item-logs-e2e_automate-kubernetes_container_image"] [data-test="dashboard-add-filter-data"]');
    await expect(filterButton1).toBeVisible();
    await filterButton1.click();
    await page
      .locator(
        '[data-test="dashboard-add-condition-label-0-kubernetes_container_name"]'
      )
      .click();
    await page
      .locator('[data-test="dashboard-add-condition-condition-0"]')
      .click();
    await page.getByText("Operatorarrow_drop_down").click();
    await page.getByText("=", { exact: true }).click();
    await page.getByLabel("Value").click();
    await page.getByLabel('Value').fill('$variablename');

    await page
      .locator(
        '[data-test="dashboard-add-condition-label-1-kubernetes_container_image"]'
      )
      .click();
    await page
      .locator('[data-test="dashboard-add-condition-condition-1"]')
      .click();
    await page.getByText("Operatorarrow_drop_down").click();
    await page
      .getByRole("option", { name: "<>" })
      .locator("div")
      .nth(2)
      .click();
    await page
      .locator("div")
      .filter({ hasText: /^Value$/ })
      .nth(2)
      .click();
      await page.getByLabel('Value').fill('$variablename');

    await page.locator('[data-test="dashboard-apply"]').click();
    await page.getByText("arrow_rightQueryAutoPromQLCustom SQL").click();
    await expect(page.getByText("'$variablename'").first()).toBeVisible();
    await page
      .locator('[data-test="dashboard-panel-data-view-query-inspector-btn"]')
      .click();
    // await expect(
    //   page.getByRole("cell", {
    //     name: 'SELECT histogram(_timestamp) as "x_axis_1", count(_timestamp) as "y_axis_1" FROM "e2e_automate" WHERE kubernetes_container_name = \'ziox\' AND kubernetes_container_image <> \'ziox\' GROUP BY x_axis_1 ORDER BY x_axis_1 ASC',
    //     exact: true,
    //   })
    // ).toBeVisible();

    const cell = page.getByRole("cell", {
      name: 'SELECT histogram(_timestamp) as "x_axis_1", count(_timestamp) as "y_axis_1" FROM "e2e_automate" WHERE kubernetes_container_name = \'ziox\' AND kubernetes_container_image <> \'ziox\' GROUP BY x_axis_1 ORDER BY x_axis_1 ASC',
      exact: true,
    });
    
    // Check if the cell is visible
    await expect(cell).toBeVisible();
    
    // Verify it contains the correct text
    await expect(cell).toHaveText(
      'SELECT histogram(_timestamp) as "x_axis_1", count(_timestamp) as "y_axis_1" FROM "e2e_automate" WHERE kubernetes_container_name = \'ziox\' AND kubernetes_container_image <> \'ziox\' GROUP BY x_axis_1 ORDER BY x_axis_1 ASC'
    );
    
    await page.locator("#q-portal--dialog--273").getByRole("button").click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("test");
    await page.locator('[data-test="dashboard-panel-save"]').click();
  });

  
  test("Should  apply the  filter group inside group", async ({
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
    await page.getByRole("tab", { name: "Variables" }).click();
    await page.getByRole("button", { name: "Add Variable" }).click();
    await page
      .locator("div")
      .filter({ hasText: /^Name \*$/ })
      .nth(2)
      .click();

    // await page.getByLabel('Name *').click();
    await page.getByLabel("Name *").fill("variablename");

    await page
      .locator("label")
      .filter({ hasText: "Stream Type *arrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "logs" }).click();
    await page
      .locator("label")
      .filter({ hasText: "Stream *arrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();
    await page.getByText("Field *arrow_drop_down").click();
    await page
      .getByRole("option", { name: "kubernetes_container_name" })
      .locator("div")
      .nth(2)
      .click();
    await page.getByRole("button", { name: "Save" }).click();

    await page.waitForTimeout(3000);

    const button = page.locator(
      '[data-test="dashboard-if-no-panel-add-panel-btn"]'
    );
    await expect(button).toBeVisible();

    await page.waitForTimeout(3000);
    await button.click();

    // await page
    //   .locator("label")
    //   .filter({ hasText: "Streamarrow_drop_down" })
    //   .locator("i")
    //   .click();

      await page.waitForTimeout(2000);

    await page.locator('[data-test="index-dropdown-stream"]').waitFor({ state: 'visible' });
    await page.locator('[data-test="index-dropdown-stream"]').click();

    // await page.getByText('Streamarrow_drop_down').click();
//     const streamDropdown = page.locator('[data-test="index-dropdown-stream"]').click();
// await expect(streamDropdown).toBeVisible();
// await streamDropdown.click();

     await page.waitForTimeout(2000);

    await page.getByRole("option", { name: "e2e_automate", exact: true }).click();

    // await page.waitForTimeout(2000);

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-_timestamp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
     await page.locator('[data-test="dashboard-apply"]').click();

     await page.locator('[data-test="date-time-btn"]').click();
     await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
     await page.locator('[data-test="date-time-apply-btn"]').click();

    await page.waitForTimeout(3000);

    await page.locator('[data-test="dashboard-add-condition-add"]').click();
    await page.getByText('Add Group').click();
    await page.locator('[data-test="dashboard-add-condition-label-0-_timestamp"]').click();
    await page.getByText('Filters on Fieldarrow_drop_down').click();
    await page.getByRole('option', { name: 'kubernetes_container_name' }).click();
    await page.locator('[data-test="dashboard-add-condition-condition-0"]').click();
    await page.getByText('Operatorarrow_drop_down').click();
    await page.getByText('=', { exact: true }).click();
    await page.getByLabel('Value').click();
    await page.getByLabel('Value').fill('$variablename');

    await page.locator('div').filter({ hasText: /^kubernetes_container_namearrow_drop_downcloseadd$/ }).locator('[data-test="dashboard-add-condition-add"]').click();
    await page.locator('div').filter({ hasText: 'Add Group' }).nth(3).click();
    await page.locator('[data-test="dashboard-add-condition-label-0-_timestamp"]').click();
    await page.getByText('Filters on Fieldarrow_drop_down').click();
    await page.getByRole('option', { name: 'kubernetes_container_image' }).click();
    await page.locator('[data-test="dashboard-add-condition-condition-0"]').click();
    await page.getByText('Operatorarrow_drop_down').click();
    await page.getByRole('option', { name: '<>' }).locator('div').nth(2).click();

    await page.getByLabel('Value').click();
    await page.getByLabel('Value').fill('$variablename');
    
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.getByText('<blank>variablenamearrow_drop_down').click();
    await page.getByRole('option', { name: 'ziox' }).click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.getByText('arrow_rightQueryAutoPromQLCustom SQL').click();
    await expect(page.getByText('\'$variablename\'').first()).toBeVisible();
    await page.locator('[data-test="dashboard-panel-data-view-query-inspector-btn"]').click();
    await expect(page.getByRole('cell', { name: 'SELECT histogram(_timestamp) as "x_axis_1", count(_timestamp) as "y_axis_1" FROM "e2e_automate" WHERE (kubernetes_container_name = \'ziox\' AND (kubernetes_container_image <> \'ziox\')) GROUP BY x_axis_1 ORDER BY x_axis_1 ASC', exact: true })).toBeVisible();
    await page.locator('#q-portal--dialog--104').getByRole('button').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill('test');
    await page.locator('[data-test="dashboard-panel-save"]').click();


    });
  });


