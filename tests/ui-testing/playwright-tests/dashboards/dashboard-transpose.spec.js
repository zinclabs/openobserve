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
  test("should verify that the transpose toggle button is working correctly", async ({
    page,
    browser,
  }) => {
    // Navigate to dashboards
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Create a new dashboard
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add panel to the dashboard
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("bhj");
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="selected-chart-table-item"] img').click();
    await page.getByRole("cell", { name: "Kubernetes Container Name" }).click();
    await page.locator('[data-test="dashboard-sidebar"]').click();
    await page
      .locator('[data-test="dashboard-config-table_transpose"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
  });

  test("should display the correct data before and after transposing in the table chart", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Create a new dashboard
    await page.locator('[data-test="dashboard-add"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add panel to the dashboard
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();
    await page
      .locator('[data-test="dashboard-x-item-_timestamp-remove"]')
      .click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_namespace_name"] [data-test="dashboard-add-x-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("test");
    await page.locator('[data-test="date-time-btn"]').click();

    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.waitForTimeout(2000);

    // Select table chart and perform transpose
    await page.locator('[data-test="selected-chart-table-item"] img').click();
    await page.getByRole("cell", { name: "Kubernetes Container Name" }).click();
    await page.locator('[data-test="dashboard-sidebar"]').click();
    await page
      .locator('[data-test="dashboard-config-table_transpose"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(2000);

    // Validate data consistency before and after transpose
    await validateTableDataBeforeAndAfterTranspose(page);

    // Helper function to validate table data before and after transposing
    // Helper function to dynamically transpose data and validate it
    async function validateTableDataBeforeAndAfterTranspose(page) {
      // Step 1: Capture headers and initial data from the table
      const headers = await page.$$eval(
        '[data-test="dashboard-panel-table"] thead tr th',
        (headerCells) =>
          headerCells.map((cell) =>
            cell.textContent.trim().replace(/^arrow_upward/, "")
          ) // Remove "arrow_upward" prefix
      );

      const initialData = await page.$$eval(
        '[data-test="dashboard-panel-table"] tbody tr',
        (rows) =>
          rows
            .map((row) =>
              Array.from(row.querySelectorAll("td"), (cell) =>
                cell.textContent.trim()
              )
            )
            .filter((row) => row.length > 0 && row.some((cell) => cell !== ""))
      );

      // Step 2: Perform transpose by simulating the transpose button click
      await page
        .locator('[data-test="dashboard-config-table_transpose"] div')
        .nth(2)
        .click();
      await page.locator('[data-test="dashboard-apply"]').click();
      await page.waitForTimeout(2000);

      // Step 3: Capture transposed data from the table
      const transposedData = await page.$$eval(
        '[data-test="dashboard-panel-table"] tr',
        (rows) =>
          rows
            .map((row) =>
              Array.from(row.querySelectorAll("td"), (cell) =>
                cell.textContent.trim()
              )
            )
            .filter((row) => row.length > 0 && row.some((cell) => cell !== ""))
      );

      // Step 4: Flatten `initialData` by pairing each namespace header with its value, excluding the empty namespace
      const flattenedInitialData = headers
        .slice(1)
        .map((namespace, index) => [namespace, initialData[0][index + 1]]);

      // Step 5: Sort both `flattenedInitialData` and `transposedData` for comparison
      const sortedFlattenedInitialData = flattenedInitialData.sort((a, b) =>
        a[0].localeCompare(b[0])
      );
      const sortedTransposedData = transposedData.sort((a, b) =>
        a[0].localeCompare(b[0])
      );

      // Step 6: Directly compare sorted arrays
      expect(sortedTransposedData).toEqual(sortedFlattenedInitialData);
    }
  });

  test("should verify that the transpose toggle button is working correctly ", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Create a new dashboard
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add panel to the dashboard
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_docker_id"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="selected-chart-table-item"] img').click();
    await page.locator('[data-test="dashboard-sidebar"]').click();
    await page
      .locator('[data-test="dashboard-config-table_transpose"] div')
      .first()
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page
      .locator('[data-test="dashboard-config-table_transpose"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await expect(
      page.getByRole("cell", { name: "Kubernetes Container Name" })
    ).toBeVisible();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("test");
    await page.locator('[data-test="dashboard-panel-save"]').click();
  });

  test("should verify that when dynamic columns are enabled, the VRL function should display correctly", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Create a new dashboard
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add panel to the dashboard
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_docker_id"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-sidebar"]').click();
    await page.locator('[data-test="selected-chart-table-item"] img').click();
    await page
      .locator('[data-test="dashboard-config-table_dynamic_columns"] div')
      .nth(2)
      .click();
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] div')
      .nth(2)
      .click();
    await page
      .locator(
        "#fnEditor > .monaco-editor > .overflow-guard > div:nth-child(2) > .lines-content > .view-lines > .view-line"
      )
      .click();
    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrl=100");

    await page.waitForTimeout(2000);

    await page
      .locator('[data-test="dashboard-config-table_dynamic_columns"] div')
      .nth(2)
      .click();

    await page
      .locator('[data-test="dashboard-config-table_dynamic_columns"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await expect(
      page
        .locator('[data-test="dashboard-panel-table"]')
        .getByRole("cell", { name: "vrl" })
    ).toBeVisible();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("test");
    await page.locator('[data-test="dashboard-panel-save"]').click();
  });

  test("should not show an error when both the Transpose and Dynamic Column toggle buttons are enabled", async ({
    page,
  }) => {
    // Set up listener to catch console errors
    let errorMessage = "";
    page.on("console", (msg) => {
      if (msg.type() === "error") {
        errorMessage += msg.text() + "\n";
      }
    });

    // Navigate to dashboard creation and configuration steps
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Create a new dashboard
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add panel to the dashboard
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.locator('[data-test="dashboard-sidebar"]').click();
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] div')
      .nth(2)
      .click();
    await page
      .locator(
        "#fnEditor > .monaco-editor > .overflow-guard > div:nth-child(2) > .lines-content > .view-lines > .view-line"
      )
      .click();
    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrl=100");

    await page.locator('[data-test="selected-chart-table-item"] img').click();
    await page
      .locator('[data-test="dashboard-config-table_dynamic_columns"] div')
      .nth(2)
      .click();
    await page
      .locator('[data-test="dashboard-config-table_transpose"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("test");
    await page.locator('[data-test="dashboard-panel-save"]').click();

    // Assert no error occurred
    expect(errorMessage).toBe("");
  });
});
