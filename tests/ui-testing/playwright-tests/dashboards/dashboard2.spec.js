import { test, expect } from "../baseFixtures";
import logData from "../../cypress/fixtures/log.json";
import logsdata from "../../../test-data/logs_data.json";

const randomDashboardName =
  "Dashboard_" + Math.random().toString(36).substr(2, 9);

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


  test("should add the breakdown field to the dashboard using Drag and Drop and +B, and allow the user to cancel the action", async ({
    page,
  }) => {
    // Navigate to dashboards
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Add a new dashboard
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add a panel to the dashboard
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();

    // Add fields to the chart
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .getByText("drag_indicatortext_fields kubernetes_container_image")
      .click();
    await page
      .getByText("drag_indicatortext_fields kubernetes_container_image")
      .click();
    await page
      .getByText("drag_indicatortext_fields kubernetes_container_image")
      .click();

    // Set the date-time range and apply changes
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-4-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(100);

    // Verify the breakdown field removal
    await page.locator(
      '[data-test="dashboard-b-item-kubernetes_labels_app_kubernetes_io_component-remove"]'
    );

    // Verify adding a new breakdown field
    await expect(
      page.locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_labels_operator_prometheus_io_name"] [data-test="dashboard-add-b-data"]'
      )
    ).toBeVisible();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_labels_operator_prometheus_io_name"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    // await page.locator('[data-test="dashboard-apply"]');
    await page.locator('[data-test="dashboard-apply"]').click();

    // Save the panel with a new name
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("Dash_01");
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-save"]').click();

    // Delete the panel and confirm
    await page
      .locator('[data-test="dashboard-edit-panel-Dash_01-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await expect(page.getByText("Are you sure you want to")).toHaveText(
      "Are you sure you want to delete this Panel?"
    );
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("should add and cancel the breakdown field with different times and timezones and ensure it displays the correct output", async ({
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

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page
      .getByRole("option", { name: "e2e_automate" })
      .locator("div")
      .nth(2)
      .click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_image"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(100);

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(100);
    await page
      .locator(
        '[data-test="dashboard-b-item-kubernetes_container_hash-remove"]'
      )
      .click();
    await expect(
      page.getByText("Chart Configuration / Variables has been updated")
    ).toBeVisible();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(100);

    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page
      .locator('[data-test="dashboard-panel-name"]')
      .fill("Dash_Breakdown");
    await page.locator('[data-test="dashboard-panel-save"]').click();
    await page.waitForTimeout(1000);

    // Delete the panel and confirm
    await page
      .locator('[data-test="dashboard-edit-panel-Dash_Breakdown-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await expect(page.getByText("Are you sure you want to")).toHaveText(
      "Are you sure you want to delete this Panel?"
    );
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("should update the breakdown field correctly to match the existing one according to the chart type when changing the chart type.", async ({
    page,
  }) => {
    // The existing added fields in the dropdown should adjust correctly according to the new chart type select

    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Add a new dashboard
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add a panel to the dashboard
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page
      .getByRole("option", { name: "e2e_automate" })
      .locator("div")
      .nth(2)
      .click();

    // Add fields to the chart
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_image"] [data-test="dashboard-add-y-data"]'
      )
      .click();

    // Set the date-time range and apply changes
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(100);

    // Verify the initial chart rendering
    //  await expect(page.locator('[data-test="chart-renderer"] canvas')).toBeVisible();
    await page
      .locator('[data-test="dashboard-b-item-kubernetes_container_hash"]')
      .click();
    await expect(
      page.locator('[data-test="dashboard-b-item-kubernetes_container_hash"]')
    ).toBeVisible();

    // Area chart
    await page.locator('[data-test="selected-chart-area-item"] img').click();
    await expect(
      page.locator('[data-test="dashboard-b-item-kubernetes_container_hash"]')
    ).toBeVisible();

    // Area stacked
    const graphLocatorAreaStacked = page.locator(
      '[data-test="selected-chart-area-stacked-item"]'
    );
    await expect(graphLocatorAreaStacked).toBeVisible();

    // H-bar chart
    const graphLocatorHBar = page.locator(
      '[data-test="selected-chart-h-bar-item"]'
    );
    await expect(graphLocatorHBar).toBeVisible();

    // Scatter chart
    await page.locator('[data-test="selected-chart-scatter-item"] img').click();
    const graphLocatorScatter = page.locator(
      '[data-test="selected-chart-scatter-item"]'
    ); // Replace with the actual selector for the graph
    await expect(graphLocatorScatter).toBeVisible();

    // H-stacked chart
    await page
      .locator('[data-test="selected-chart-h-stacked-item"] img')
      .click();
    const graphLocatorHStacked = page.locator(
      '[data-test="selected-chart-h-stacked-item"]'
    ); // Replace with the actual selector for the graph
    await expect(graphLocatorHStacked).toBeVisible();

    // Stacked chart
    await page.locator('[data-test="selected-chart-stacked-item"] img').click();
    const graphLocatorStacked = page.locator(
      '[data-test="selected-chart-stacked-item"]'
    ); // Replace with the actual selector for the graph
    await expect(graphLocatorStacked).toBeVisible();

    // Save the dashboard panel
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("Dashboard");
    await page.locator('[data-test="dashboard-panel-save"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    // Switch to Bar chart and apply changes
    await page.locator('[data-test="selected-chart-bar-item"] img').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-save"]').click();
    await page.waitForTimeout(200);

    // Delete the panel and confirm
    await page
      .locator('[data-test="dashboard-edit-panel-Dashboard-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await expect(page.getByText("Are you sure you want to")).toHaveText(
      "Are you sure you want to delete this Panel?"
    );
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("should create the panel successfully after adding a breakdown", async ({
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
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_image"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(100);

    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("Dash1");
    await page.locator('[data-test="dashboard-panel-save"]').click();

    // Delete the panel and confirm
    await page
      .locator('[data-test="dashboard-edit-panel-Dash1-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await expect(page.getByText("Are you sure you want to")).toHaveText(
      "Are you sure you want to delete this Panel?"
    );
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("should clear the selections after adding a breakdown and refreshing the page.", async ({
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
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_host"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-absolute-tab"]').click();
    await page
      .locator("button")
      .filter({ hasText: "chevron_left" })
      .first()
      .click();

    await page.getByRole("button", { name: "9" }).last().click();
    await page.getByRole("button", { name: "16" }).last().click();
    await page.locator('[data-test="chart-renderer"] div').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.reload();
    await page.waitForTimeout(1000);

    page.once("dialog", (dialog) => {
      console.log(`Dialog message: ${dialog.message()}`);
      dialog.dismiss().catch(() => {});
    });
    // await expect(page.locator('[data-test="chart-renderer"] canvas')).toBeVisible();
    //  await page.waitForTimeout(1000);
    await expect(page.locator('[data-test="no-data"]')).toBeVisible();
  });

  test("should display the correct output when changing relative and absolute times with different timezones after adding a breakdown", async ({
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
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page
      .locator("label")
      .filter({ hasText: "Timezonearrow_drop_down" })
      .locator("i")
      .click();
    await page
      .locator('[data-test="datetime-timezone-select"]')
      .fill("Asia/Dhaka");
    await page.getByText("Asia/Dhaka").click();
    await page.locator('[data-test="no-data"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-absolute-tab"]').click();
    await page
      .locator("button")
      .filter({ hasText: "chevron_left" })
      .first()
      .click();

    await page.getByRole("button", { name: "8" }).last().click();
    await page.getByRole("button", { name: "16" }).last().click();

    await page.locator("#date-time-menu").getByText("arrow_drop_down").click();
    await page.locator('[data-test="datetime-timezone-select"]').click();
    await page.locator('[data-test="datetime-timezone-select"]').fill("Asia/c");
    await page.getByText("Asia/Calcutta", { exact: true }).click();
    // await page.locator('.layout-panel-container > .flex').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("Dashboard");
    await page.locator('[data-test="dashboard-panel-save"]').click();

    await page
      .locator('[data-test="dashboard-edit-panel-Dashboard-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await expect(page.getByText("Are you sure you want to")).toHaveText(
      "Are you sure you want to delete this Panel?"
    );
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("should redirect to the list of dashboard pages when discarding changes", async ({
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
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_image"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_docker_id"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();

    page.once("dialog", (dialog) => {
      console.log(`Dialog message: ${dialog.message()}`);
      dialog.dismiss().catch(() => {});
    });
    await page.locator('[data-test="dashboard-panel-discard"]').click();
  });

  test('should plot the data when adding a "Sort by" filter, a breakdown, and other required fields', async ({
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
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page
      .locator("label")
      .filter({ hasText: "Timezonearrow_drop_down" })
      .locator("i")
      .click();
    await page.locator('[data-test="datetime-timezone-select"]').fill("Asia/c");
    await page.getByText("Asia/Calcutta", { exact: true }).click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(100);

    await expect(
      page.locator('[data-test="dashboard-b-item-kubernetes_container_name"]')
    ).toBeVisible();
    await page
      .locator('[data-test="dashboard-b-item-kubernetes_container_name"]')
      .click();
    await page.locator('[data-test="dashboard-sort-by-item-asc"]').click(); //Filter A to C

    await page
      .locator('[data-test="chart-renderer"] canvas')
      .last()
      .click({
        position: {
          x: 829,
          y: 31,
        },
      });
    await page.locator('[data-test="dashboard-apply"]').click();
    await page
      .locator('[data-test="dashboard-b-item-kubernetes_container_name"]')
      .click();
    await page.locator('[data-test="dashboard-sort-by-item-desc"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("Dashboard");
    await page.locator('[data-test="dashboard-panel-save"]').click();

    await page
      .locator('[data-test="dashboard-edit-panel-Dashboard-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await expect(page.getByText("Are you sure you want to")).toHaveText(
      "Are you sure you want to delete this Panel?"
    );
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("should correctly handle and display string and numeric values when no value replacement occurs", async ({
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
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubectl_kubernetes_io_default_container"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();

    // await page.locator('[data-test="date-time-btn"]').click();
    // await page.locator('[data-test="date-time-relative-45-m-btn"]').click();

    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(100);

    
    await page.locator(".layout-panel-container > .flex").click();
    await page.getByText("expand_allConfig").click();
    await page.locator(".q-pa-none > .q-list > div").first().click();
    await page
      .locator('[data-test="dashboard-config-no-value-replacement"]')
      .click();
    await page
      .locator('[data-test="dashboard-config-no-value-replacement"]')
      .fill("NA");
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("Dashboard");
    await page.locator('[data-test="dashboard-panel-save"]').click();

    await page
      .locator('[data-test="dashboard-edit-panel-Dashboard-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await expect(page.getByText("Are you sure you want to")).toHaveText(
      "Are you sure you want to delete this Panel?"
    );
    await page.locator('[data-test="confirm-button"]').click();
  });
});