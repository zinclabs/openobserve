import { test, expect } from "../baseFixtures";
import logData from "../../cypress/fixtures/log.json";
import logsdata from "../../../test-data/logs_data.json";

test.describe.configure({ mode: "parallel" });

async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"]);
  await page.waitForTimeout(1000);

  if (await page.getByText("Login as internal user").isVisible()) {
    await page.getByText("Login as internal user").click();
  }

  await page
    .locator('[data-cy="login-user-id"]')
    .fill(process.env["ZO_ROOT_USER_EMAIL"]);
  //Enter Password
  await page
    .locator('[data-cy="login-password"]')
    .fill(process.env["ZO_ROOT_USER_PASSWORD"]);
  await page.locator('[data-cy="login-sign-in"]').click();
  await page.waitForTimeout(4000);
  await page.goto(process.env["ZO_BASE_URL"]);
}

const selectStreamAndStreamTypeForLogs = async (page, stream) => {
  await page.waitForTimeout(4000);
  await page
    .locator('[data-test="log-search-index-list-select-stream"]')
    .click({ force: true });
  await page
    .locator("div.q-item")
    .getByText(`${stream}`)
    .first()
    .click({ force: true });
};
test.describe(" visualize UI testcases", () => {
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
    await page.waitForTimeout(5000);

    // ("ingests logs via API", () => {
    const orgId = process.env["ORGNAME"];
    const streamName = "e2e_automate";
    const basicAuthCredentials = Buffer.from(
      `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
    ).toString("base64");

    const headers = {
      Authorization: `Basic ${basicAuthCredentials}`,
      "Content-Type": "application/json",
    };

    // const logsdata = {}; // Fill this with your actual data

    // Making a POST request using fetch API
    const response = await page.evaluate(
      async ({ url, headers, orgId, streamName, logsdata }) => {
        const fetchResponse = await fetch(
          `${url}/api/${orgId}/${streamName}/_json`,
          {
            method: "POST",
            headers: headers,
            body: JSON.stringify(logsdata),
          }
        );
        return await fetchResponse.json();
      },
      {
        url: process.env.INGESTION_URL,
        headers: headers,
        orgId: orgId,
        streamName: streamName,
        logsdata: logsdata,
      }
    );

    console.log(response);
    //  });
    // const allorgs = page.waitForResponse("**/api/default/organizations**");
    // const functions = page.waitForResponse("**/api/default/functions**");
    await page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );
    const allsearch = page.waitForResponse("**/api/default/_search**");
    await selectStreamAndStreamTypeForLogs(page, logData.Stream);
    await applyQueryButton(page);
    // const streams = page.waitForResponse("**/api/default/streams**");
  });
  test("should allow adding a VRL function in the visualization chart", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-4-d-btn"]').click();

    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-visualize-toggle"]').click();

    await page.waitForTimeout(5000);
    await page
      .locator('[data-test="logs-vrl-function-editor"]')
      .first()
      .click();
    await page
      .locator("#fnEditor")
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrl12=123");

    await page.waitForTimeout(3000);

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .waitFor({ state: "visible" });

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl12"] [data-test="dashboard-add-b-data"]'
      )
      .click();

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();

    await expect(
      page.locator('[data-test="field-list-item-logs-e2e_automate-vrl12"]')
    ).toBeVisible();
  });

  test.skip('should display an error message when the VRL field is not updated after closing the "Toggle function editor"', async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-visualize-toggle"]').click();

    await page.waitForTimeout(2000);

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.waitForTimeout(5000);
    await page
      .locator('[data-test="logs-vrl-function-editor"]')
      .first()
      .click();
    await page
      .locator("#fnEditor")
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrL=1000");
    await page.waitForTimeout(3000);

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] img')
      .click();
    //await page.locator('[data-test="logs-search-bar-visualize-refresh-btn"]').click();
    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
    await page.waitForTimeout(500); // Waits for 500ms before the second click
    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();

    await page.waitForTimeout(500);

    await page
      .locator('text="There are some errors, please fix them and try again"')
      .waitFor({ state: "visible" });

    await page.locator("#q-notify").getByRole("button").click();
    await expect(page.getByText("Please update Y-Axis")).toBeVisible();
    await page.locator('[data-test="dashboard-y-item-vrl-remove"]').click();
    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
  });

  test("should not show an error when adding a VRL function field to the Breakdown, X axis, or Y axis fields", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();

    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();

    await page.locator('[data-test="logs-visualize-toggle"]').click();

    await page.waitForTimeout(5000);
    await page
      .locator('[data-test="logs-vrl-function-editor"]')
      .first()
      .click();
    await page
      .locator("#fnEditor")
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".VRL=1000");

    await page.waitForTimeout(3000);

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .waitFor({ state: "visible" });

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-b-data"]'
      )
      .click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-y-data"]'
      )
      .click();

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();

    await page
      .locator('[data-test="dashboard-x-item-_timestamp-remove"]')
      .click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-x-data"]'
      )
      .click();

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();

    const breakdownField = await page
      .locator('[data-test="dashboard-b-item-vrl"]')
      .isVisible();
    const yAxisField = await page
      .locator('[data-test="dashboard-y-item-vrl"]')
      .isVisible();
    const xAxisField = await page
      .locator('[data-test="dashboard-x-item-vrl"]')
      .isVisible();

    expect(breakdownField).toBe(true);
    expect(yAxisField).toBe(true);
    expect(xAxisField).toBe(true);
  });

  test("should display an error message if an invalid VRL function is added", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-visualize-toggle"]').click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_image"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
    await page.waitForTimeout(5000);
    await page
      .locator('[data-test="logs-vrl-function-editor"]')
      .first()
      .click();
    await page
      .locator("#fnEditor")
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrl=11abc");
    await page.waitForTimeout(2000);

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();

    // await expect(page.getByText("warningFunction error: error[")).toBeVisible();

    await page
      .locator("#fnEditor")
      .getByLabel("Editor content;Press Alt+F1")
      .press("Control+a");
    await page
      .locator("#fnEditor")
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrl=123");

    await page.waitForTimeout(3000);

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .waitFor({ state: "visible" });

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
  });

  test("should not update the search query when adding or updating a VRL field", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-visualize-toggle"]').click();

    await page.getByLabel("SQL Mode").locator("div").nth(2).click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
    await page.locator('[data-test="logs-visualize-toggle"]').click();

    await expect(
      page
        .locator('[data-test="logs-search-bar-query-editor"]')
        .getByText('SELECT * FROM "e2e_automate"')
    ).toBeVisible();
    await page.waitForTimeout(5000);
    await page
      .locator('[data-test="logs-vrl-function-editor"]')
      .first()
      .click();
    await page
      .locator("#fnEditor")
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrl=100");

    await page.waitForTimeout(3000);
    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .waitFor({ state: "visible" });

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
    await expect(
      page
        .locator('[data-test="logs-search-bar-query-editor"]')
        .getByText('SELECT * FROM "e2e_automate"')
    ).toBeVisible();
  });

  test.skip("should display an error if the VRL field is not updated from the Breakdown", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-visualize-toggle"]').click();
    await page.waitForTimeout(5000);
    await page
      .locator('[data-test="logs-vrl-function-editor"]')
      .first()
      .click();
    await page
      .locator("#fnEditor")
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrlsanity=100");

    await page.waitForTimeout(3000);

    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrlsanity"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page
      .locator("div")
      .filter({ hasText: /^\.vrlsanity=100$/ })
      .nth(3)
      .click();
    await page
      .locator("#fnEditor")
      .getByLabel("Editor content;Press Alt+F1")
      .press("Control+a");
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
  });

  test("should update the data on the chart when changing the time after applying a VRL field", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-visualize-toggle"]').click();
    await page.waitForTimeout(5000);
    await page
      .locator('[data-test="logs-vrl-function-editor"]')
      .first()
      .click();
    await page
      .locator("#fnEditor")
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrl=123");

    await page.waitForTimeout(3000);

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .waitFor({ state: "visible" });

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-4-w-btn"]').click();
    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
  });

  test("should not show an error when changing the chart type after adding a VRL function field", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-visualize-toggle"]').click();

    // Set up a flag to detect errors
    let errorDetected = false;

    // Listen for console messages and check for errors
    page.on("console", (msg) => {
      if (msg.type() === "error") {
        const errorText = msg.text();
        // Check if the error matches a known pattern (customize regex as needed)
        if (/Error|Failure|Cannot|Invalid/i.test(errorText)) {
          errorDetected = true;
        }
      }
    });

    await page.waitForTimeout(5000);
    await page
      .locator('[data-test="logs-vrl-function-editor"]')
      .first()
      .click();
    await page
      .locator("#fnEditor")
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".VRLsanity=1000");
    await page.waitForTimeout(3000);

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .waitFor({ state: "visible" });

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrlsanity"] [data-test="dashboard-add-y-data"]'
      )
      .click();

    await page
      .locator('[data-test="logs-search-bar-visualize-refresh-btn"]')
      .click();

    // Change chart types and check for errors each time
    const chartTypes = [
      '[data-test="selected-chart-area-item"] img',
      '[data-test="selected-chart-area-stacked-item"] img',
      '[data-test="selected-chart-h-bar-item"] img',
      '[data-test="selected-chart-scatter-item"] img',
      '[data-test="selected-chart-h-stacked-item"] img',
      '[data-test="selected-chart-heatmap-item"] img',
      '[data-test="selected-chart-pie-item"] img',
      '[data-test="selected-chart-table-item"] img',
      '[data-test="selected-chart-gauge-item"] img',
    ];

    for (const chartType of chartTypes) {
      await page.locator(chartType).click();

      await page.waitForTimeout(1000);
    }

    // Assertion: Fail the test if an error was detected
    expect(errorDetected).toBe(false);
  });
});
