import { expect } from '@playwright/test';
import {
  dateTimeButtonLocator, relative30SecondsButtonLocator, absoluteTabLocator,
  Past30SecondsValue
} from '../pages/CommonLocator.js';
export class DashboardPage {
  constructor(page) {
    this.page = page;
    this.dashboardName = `dashboard${Date.now()}`;
    this.dashboardsMenuItem = page.locator('[data-test="menu-link-\\/dashboards-item"]');
    this.addDashboardButton = page.locator('[data-test="dashboard-add"]');
    this.dashboardNameInput = page.locator('[data-test="add-dashboard-name"]');
    this.dashboardSubmitButton = page.locator('[data-test="dashboard-add-submit"]');
    this.savePanelButton = page.locator('[data-test="dashboard-panel-save"]');
    this.dashboardPanelNameInput = page.locator('[data-test="dashboard-panel-name"]');
    this.dateTimeButton = dateTimeButtonLocator;
    this.relative30SecondsButton = page.locator(relative30SecondsButtonLocator);
    this.absoluteTab = absoluteTabLocator;
    this.profileButton = page.locator('button').filter({ hasText: (process.env["ZO_ROOT_USER_EMAIL"]) });
    this.signOutButton = page.getByText('Sign Out');
  }
  async navigateToDashboards() {
    await this.page.waitForSelector('[data-test="menu-link-\\/dashboards-item"]');
    await this.dashboardsMenuItem.click();
    await this.page.waitForTimeout(5000);
    await expect(this.page.getByRole('main')).toContainText('Dashboards');
  }
  async createDashboard() {
    await this.page.waitForSelector('[data-test="dashboard-add"]');
    await this.addDashboardButton.click();
    await this.page.waitForTimeout(5000);
    await this.dashboardNameInput.fill(this.dashboardName);
    await this.page.waitForSelector('[data-test="dashboard-add-submit"]');
    await this.dashboardSubmitButton.click();
    await this.page.waitForTimeout(2000);
    await this.page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await this.page.waitForTimeout(3000);
    await expect(this.page.getByText("Dashboard added successfully.")).toBeVisible({
      timeout: 3000,
    });
    await this.page.locator('label').filter({ hasText: 'Streamarrow_drop_down' }).locator('i').click();
    // Refine the locator for 'e2e_automate'
    await this.page
      .locator("span")
      .filter({ hasText: /^e2e_automate$/ })
      .click();
    await this.page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await this.page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_image"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await this.page.waitForSelector('[data-test="dashboard-panel-name"]');
    await this.page.locator('[data-test="dashboard-panel-name"]').click();
    await this.page.locator('[data-test="dashboard-panel-name"]').fill('AutoP');
    await this.page.locator('[data-test="dashboard-panel-name"]').press('Enter');
    await expect(this.page.locator('[data-test="dashboard-apply"]')).toBeVisible();
    await this.page.locator('[data-test="dashboard-apply"]').click();
    await this.page.waitForTimeout(5000);
  }
  async deleteDashboard() {
    await this.page.reload();
    await this.page.waitForTimeout(2000);
    await this.page.locator("//td[contains(text(),'" + this.dashboardName + "')]/following-sibling::td[@class='q-td text-center']/child::button[@data-test='dashboard-delete']").click({ force: true });
    await this.page.waitForTimeout(2000);
    await this.page.locator('[data-test="confirm-button"]:visible').click();
    await expect(this.page.getByRole('alert')).toContainText('Dashboard deleted successfully.');
  }
  async setTimeToPast30Seconds() {
    // Set the time filter to the last 30 seconds
    await this.page.locator(this.dateTimeButton).click();
    await this.relative30SecondsButton.click();
  }
  async verifyTimeSetTo30Seconds() {
    // Verify that the time filter displays "Past 30 Seconds"
    await expect(this.page.locator(this.dateTimeButton)).toContainText(Past30SecondsValue);
  }
  async setDateTime() {
    await expect(this.page.locator(this.dateTimeButton)).toBeVisible();
    await this.page.locator(this.dateTimeButton).click();
    await this.page.locator(this.absoluteTab).click();
    await this.page.waitForTimeout(2000);
  }
  async fillTimeRange(startTime, endTime) {
    await this.page.getByRole('button', { name: '1', exact: true }).click();
    await this.page.getByLabel('access_time').first().fill(startTime);
    await this.page.getByRole('button', { name: '1', exact: true }).click();
    await this.page.getByLabel('access_time').nth(1).fill(endTime);
  }
  async verifyDateTime(startTime, endTime) {
    await expect(this.page.locator(this.dateTimeButton)).toContainText(`${startTime} - ${endTime}`);
  }

  async dashboardPageDefaultMultiOrg() {
    await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
    await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();
  }

  async dashboardPageURLValidation() {
    // TODO: Fix this test
    // await expect(this.page).not.toHaveURL(/default/);
  }

  async dashboardURLValidation() {
    await expect(this.page).toHaveURL(/dashboard/);
  }

  async signOut() {
    await this.profileButton.click();
    await this.signOutButton.click();
  }
}


