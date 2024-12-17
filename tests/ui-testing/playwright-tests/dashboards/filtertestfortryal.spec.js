
import { test, expect } from "../baseFixtures";
import logData from "../../cypress/fixtures/log.json";
import logsdata from "../../../test-data/logs_data.json";
import { login } from '../../pages/dashLogin.js';
import { ingestion ,removeUTFCharacters } from '../../pages/dashIngestion.js';
import { waitForDashboardPage , applyQueryButton } from '../../pages/dashCreation.js';

const randomDashboardName =
  "Dashboard_" + Math.random().toString(36).substr(2, 9);

test.describe.configure({ mode: "parallel" });

test.describe("dashboard UI testcases", () => {
  test.beforeEach(async ({ page }) => {
    console.log("running before each");
    await login(page);
    await page.waitForTimeout(1000);
    await ingestion(page);
    await page.waitForTimeout(2000);

    const orgNavigation = page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );
    await orgNavigation;
  });


  test("should create a duplicate of the dashboard", async ({ page }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await expect(page.getByText("Dashboard added successfully.")).toBeVisible({
      timeout: 30000,
    });

    await page.locator('[data-test="dashboard-back-btn"]').click();

    await expect(
      page
        .getByRole("row", { name: `01 ${randomDashboardName}` })
        .locator('[data-test="dashboard-duplicate"]')
    ).toBeVisible();
    await page
      .getByRole("row", { name: `01 ${randomDashboardName}` })
      .locator('[data-test="dashboard-duplicate"]')
      .click();
  });



});
