import { test, expect } from './baseFixtures';
import logData from "../cypress/fixtures/log.json";
import logsdata from "../../test-data/logs_data.json";
import { toZonedTime } from "date-fns-tz";
import { LogsPage } from '../pages/logsPage.js';

test.describe.configure({ mode: "parallel" });
const folderName = `Folder ${Date.now()}`;
const dashboardName = `AutomatedDashboard${Date.now()}`;

async function login(page) {
 
  await page.goto(process.env["ZO_BASE_URL"]);
  if (await page.getByText('Login as internal user').isVisible()) {
    await page.getByText('Login as internal user').click();
}
  console.log("ZO_BASE_URL", process.env["ZO_BASE_URL"]);
  await page.waitForTimeout(1000);

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
  const response = await page.evaluate(async ({ url, headers, orgId, streamName, logsdata }) => {
    const fetchResponse = await fetch(`${url}/api/${orgId}/${streamName}/_json`, {
      method: 'POST',
      headers: headers,
      body: JSON.stringify(logsdata)
    });
    return await fetchResponse.json();
  }, {
    url: process.env.INGESTION_URL,
    headers: headers,
    orgId: orgId,
    streamName: streamName,
    logsdata: logsdata
  });
  console.log(response);
}


test.describe("Sanity testcases", () => {
  let logsPage;
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
  }

  test.beforeEach(async ({ page }) => {
    await login(page);
    logsPage = new LogsPage(page);
    await page.waitForTimeout(1000)
    await ingestion(page);
    await page.waitForTimeout(2000)

    await page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );
    const allsearch = page.waitForResponse("**/api/default/_search**");
    await logsPage.selectStreamAndStreamTypeForLogs("e2e_automate"); 
    await applyQueryButton(page);
    // const streams = page.waitForResponse("**/api/default/streams**");
  });

 
  test("should change settings successfully", async ({ page }) => {
    await page.waitForTimeout(2000);
    await page.locator('[data-test="menu-link-settings-item"]').click();
    await page.waitForTimeout(2000);
    await page.getByText("General SettingsScrape").click();
    await page.getByRole("tab", { name: "General Settings" }).click();
    await page.getByLabel("Scrape Interval (In Seconds) *").fill("16");
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.getByText("Organization settings updated").click();
  });

  test("should display results on click refresh stats", async ({ page }) => {
    await page.locator('[data-test="menu-link-\\/streams-item"]').click();
    await page.locator('[data-test="log-stream-refresh-stats-btn"]').click();
    page.reload();
    await page.getByRole("cell", { name: "01", exact: true }).click();
  });

  test("should display pagination for schema", async ({ page }) => {
    await page.waitForTimeout(2000);
    await page
      .getByText("fast_rewind12345fast_forward50arrow_drop_down")
      .click();
    await page.getByText("fast_rewind1/2fast_forward").click();
    await page
      .locator('[data-test="logs-page-fields-list-pagination-nextpage-button"]')
      .click();
    await page
      .locator(
        '[data-test="logs-page-fields-list-pagination-previouspage-button"]'
      )
      .click();
  });

  test("should display pagination when histogram is off and clicking and closing the result", async ({
    page,
  }) => {
    await page
      .locator('[data-test="logs-search-bar-show-histogram-toggle-btn"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="log-table-column-0-source"]').click();
    await page.locator('[data-test="close-dialog"]').click();
    await page
      .getByText("fast_rewind12345fast_forward50arrow_drop_down")
      .click();
  });

  test("should display pagination when only SQL is on clicking and closing the result", async ({
    page,
  }) => {
    await page
      .locator('[data-test="logs-search-bar-show-histogram-toggle-btn"] div')
      .nth(2)
      .click();
    await page.getByLabel("SQL Mode").locator("div").first().click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="log-table-column-1-_timestamp"]').click();
    await page.locator('[data-test="close-dialog"]').click();
    await page
      .getByText("fast_rewind12345fast_forward50arrow_drop_down")
      .click();
  });

  test(" should display histogram in sql mode", async ({ page }) => {
    await page
      .locator('[data-test="logs-search-result-bar-chart"] canvas')
      .click({
        position: {
          x: 182,
          y: 66,
        },
      });
    await page.getByLabel("SQL Mode").locator("div").nth(2).click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    // await page.getByRole('heading', { name: 'Error while fetching' }).click();

    await expect(
      page.getByRole("heading", { name: "Error while fetching" })
    ).not.toBeVisible();
    await page
      .locator('[data-test="logs-search-result-bar-chart"] canvas')
      .click({
        position: {
          x: 182,
          y: 66,
        },
      });
  });

  test("should display results when SQL+histogram is on and then stream is selected", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/-item"]').click();
    await page.locator('[data-test="menu-link-\\/logs-item"]').click();
    await page
      .locator(
        "#fnEditor > .monaco-editor > .overflow-guard > .monaco-scrollable-element > .lines-content > .view-lines"
      )
      .click();
    await page.getByLabel("SQL Mode").locator("div").nth(2).click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page
      .locator(
        '[data-test="log-table-column-0-_timestamp"] [data-test="table-row-expand-menu"]'
      )
      .click();
  });

  const getHeaders = () => {
    const basicAuthCredentials = Buffer.from(
      `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
    ).toString("base64");

    return {
      Authorization: `Basic ${basicAuthCredentials}`,
      "Content-Type": "application/json",
    };
  };

  // Helper function to get ingestion URL
  const getIngestionUrl = (orgId, streamName) => {
    return `${process.env.INGESTION_URL}/api/${orgId}/${streamName}/_json`;
  };

  // Helper function to send POST request
  const sendRequest = async (page, url, payload, headers) => {
    return await page.evaluate(
      async ({ url, headers, payload }) => {
        const response = await fetch(url, {
          method: "POST",
          headers: headers,
          body: JSON.stringify(payload),
        });
        return await response.json();
      },
      { url, headers, payload }
    );
  };

  test.skip("should check JSON responses for successful:1 with timestamp 15 mins before", async ({
    page,
  }) => {
    const orgId = process.env["ORGNAME"];
    const streamName = "e2e_automate";
    const headers = getHeaders();
    const ingestionUrl = getIngestionUrl(orgId, streamName);

    // First payload
    const payload1 = [
      {
        level: "info",
        job: "test",
        log: "test message for openobserve",
        e2e: "1",
      },
    ];

    // Second payload with timestamp 15 minutes before
    const timestamp = Date.now() - 15 * 60 * 1000; // 15 minutes before
    const payload2 = [
      {
        level: "info",
        job: "test",
        log: "test message for openobserve",
        e2e: "1.1",
        _timestamp: timestamp,
      },
    ];

    // Sending first request
    const response1 = await sendRequest(page, ingestionUrl, payload1, headers);
    console.log(response1);

    // Sending second request
    const response2 = await sendRequest(page, ingestionUrl, payload2, headers);
    console.log(response2);

    // Assertions
    expect(response1.status[0].successful).toBe(1);
    expect(response2.status[0].successful).toBe(1);
  });

test.skip("should display error if timestamp past the ingestion time limit", async ({
    page,
  }) => {
    const orgId = process.env["ORGNAME"];
    const streamName = "e2e_automate";
    const headers = getHeaders();
    const ingestionUrl = getIngestionUrl(orgId, streamName);

    // First payload
    const payload1 = [
      {
        level: "info",
        job: "test",
        log: "test message for openobserve",
        e2e: "1",
      },
    ];

    // Second payload with timestamp 6 hours before
    const timestamp = Date.now() - 6 * 60 * 60 * 1000; // 6 hours before
    const payload2 = [
      {
        level: "info",
        job: "test",
        log: "test message for openobserve",
        e2e: "1.1",
        _timestamp: timestamp,
      },
    ];

    // Sending first request
    const response1 = await sendRequest(page, ingestionUrl, payload1, headers);
    console.log(response1);

    // Sending second request
    const response2 = await sendRequest(page, ingestionUrl, payload2, headers);
    console.log(response2);

    // Assertions
    expect(response1.status[0].successful).toBe(1);
    expect(response2.status[0].successful).toBe(0);
    expect(response2.status[0].failed).toBe(1);
    expect(response2.status[0].error).toBe(
      "Too old data, only last 5 hours data can be ingested. Data discarded. You can adjust ingestion max time by setting the environment variable ZO_INGEST_ALLOWED_UPTO=<max_hours>"
    );
  });

  const formatDate = (date) => {
    const year = String(date.getFullYear());
    const month = String(date.getMonth() + 1).padStart(2, "0");
    const day = String(date.getDate()).padStart(2, "0");
    const hours = String(date.getHours()).padStart(2, "0");
    const minutes = String(date.getMinutes()).padStart(2, "0");
    const seconds = String(date.getSeconds()).padStart(2, "0");
    return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
  };
  test.skip("should compare time displayed in table dashboards after changing timezone and then delete it ", async ({ page }) => {
    const orgId = process.env["ORGNAME"];
    const streamName = "e2e_tabledashboard";
    const headers = getHeaders();
    const ingestionUrl = getIngestionUrl(orgId, streamName);
    const timestamp = parseInt(Date.now() / 10000) * 10000 - 10 * 60 * 1000; // 10 minutes before
    // First payload
    const payload1 = [
      {
        level: "info",
        job: "test",
        log: "test message for openobserve",
        e2e: "1",
        _timestamp: timestamp,
      },
    ];

    // Sending first request
    const response1 = await sendRequest(page, ingestionUrl, payload1, headers);
    console.log(response1);

    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await page.waitForTimeout(5000);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.waitForTimeout(5000);
    await page.locator('[data-test="add-dashboard-name"]').click();

    await page.locator('[data-test="add-dashboard-name"]').fill(dashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.waitForTimeout(2000);
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page.waitForTimeout(3000);
    await page.locator('[data-test="selected-chart-table-item"] img').click();
    await page.locator('[data-test="index-dropdown-stream"]').click();
    await page
      .locator('[data-test="index-dropdown-stream"]')
      .fill("e2e_tabledashboard");
    await page.waitForTimeout(4000);
    await page.getByRole("option", { name: "e2e_tabledashboard" }).click({ force: true });
    await page.waitForTimeout(6000);

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_tabledashboard-e2e"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_tabledashboard-job"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("sanitydash");
    await page.waitForTimeout(2000);
    await page.locator('[data-test="dashboard-panel-save"]').click();
    await page.waitForTimeout(2000);

    // // Change timezone to Asia/Calcutta
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="datetime-timezone-select"]').click();
    await page
      .locator('[data-test="datetime-timezone-select"]')
      .fill("Asia/Calcutta");
    await page.getByText("Asia/Calcutta", { exact: true }).click();
    await page.waitForTimeout(200);

    // NOTE: pass selected timezone
    const calcuttaTime = toZonedTime(new Date(timestamp), "Asia/Calcutta");
    const displayedTimestampCalcutta = formatDate(calcuttaTime);
    console.log(displayedTimestampCalcutta);
    await page.waitForTimeout(2000);
    // Verify the displayed time in Asia/Calcutta
    const timeCellCalcutta = await page
      .getByRole("cell", { name: displayedTimestampCalcutta })
      .textContent();
    expect(timeCellCalcutta).toBe(displayedTimestampCalcutta);
    await page.waitForTimeout(1000);
    // Change timezone to Europe/Zurich
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="datetime-timezone-select"]').click();
    await page
      .locator('[data-test="datetime-timezone-select"]')
      .fill("Europe/Zurich");
    await page.getByText("Europe/Zurich", { exact: true }).click();
    await page.waitForTimeout(200);

    // Convert the timestamp to the required format in Europe/Zurich
    const zurichTime = toZonedTime(new Date(timestamp), "Europe/Zurich");
    const displayedTimestampZurich = formatDate(zurichTime);
    console.log(displayedTimestampZurich);

    // Verify the displayed time in Europe/Zurich
    const timeCellZurich = await page
      .getByRole("cell", { name: displayedTimestampZurich })
      .textContent();
    expect(timeCellZurich).toBe(displayedTimestampZurich);
    await page.waitForTimeout(2000);
    await page
      .locator('[data-test="dashboard-edit-panel-sanitydash-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await page.locator('[data-test="confirm-button"]').click();
    await page
      .locator("#q-notify div")
      .filter({ hasText: "check_circlePanel deleted" })
      .nth(3)
      .click();
    await page.locator('[data-test="dashboard-back-btn"]');
    await page.locator('[data-test="dashboard-back-btn"]').click();
    await page
      .getByRole("row", { name: dashboardName })
      .locator('[data-test="dashboard-delete"]')
      .click();
    await page.locator('[data-test="confirm-button"]').click();
  });
  test.skip('should verify search history displayed and user navigates to logs', async ({ page, context }) => {
    // Step 1: Click on the "Share Link" button
    await page.getByLabel('SQL Mode').locator('div').nth(2).click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.getByRole('button', { name: 'Search History' }).click();
    await page.locator('[data-test="search-history-date-time"]').click();
    await page.locator('[data-test="date-time-relative-6-h-btn"]').click();
    await page.getByRole('button', { name: 'Get History' }).click();
    await page.waitForTimeout(6000);
    await page.getByRole('cell', { name: 'Trace ID' }).click();
    // Locate the row using a known static value like the SQL query
    // const row = page.locator('tr:has-text("select histogram")');
    // Locate the row using a known static value, ignoring case sensitivity
    const row = page.locator('tr').filter({ hasText: /select histogram/i });
    // Click the button inside the located row
    await row.locator('button.q-btn').nth(0).click();
    await page.getByRole('button', { name: 'Logs' }).click();
    await page.locator('[data-test="logs-search-index-list"]').getByText('e2e_automate').click()
    await expect(page).toHaveURL(/stream_type=logs/)
  });

  test('should verify logs page displayed on click back button on search history page', async ({ page, context }) => {
    // Step 1: Click on the "Share Link" button
    await page.getByLabel('SQL Mode').locator('div').nth(2).click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.getByRole('button', { name: 'Search History' }).click();
    await page.locator('[data-test="search-history-date-time"]').click();
    await page.locator('[data-test="date-time-relative-6-h-btn"]').click();
    await page.locator('[data-test="search-history-alert-back-btn"]').click();
    await page.locator('[data-test="logs-search-index-list"] div').filter({ hasText: 'e2e_automate' }).nth(4).click();
    await page.waitForTimeout(2000);
    await page.locator('[data-test="log-table-column-0-_timestamp"]').click();
  });

  test('should verify user redirected to logs page when clicking stream explorer and on clicking get history, logs history displayed', async ({ page, context }) => {
    // Step 1: Click on the "Share Link" button
    await page.locator('[data-test="menu-link-\\/streams-item"]').click();
    await page.getByPlaceholder('Search Stream').click();
    await page.getByPlaceholder('Search Stream').fill('e2e_automate');
    await page.getByRole('button', { name: 'Explore' }).first().click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.getByRole('button', { name: 'Search History' }).click();
    await page.locator('[data-test="add-alert-title"]').click();
    await page.getByText('arrow_back_ios_new').first().click()
    await page.waitForTimeout(1000);

    // Use a more specific locator for 'e2e_automate' by targeting its unique container or parent element
    await page.locator('[data-test="logs-search-index-list"]').getByText('e2e_automate').click();

  });

});
