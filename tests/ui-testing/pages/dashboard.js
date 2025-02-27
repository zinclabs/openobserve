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
  }

  async Randomdashboardcreate (){

    const randomDashboardName =
  "Dashboard_" + Math.random().toString(36).substr(2, 9); 


 await this.page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
  await waitForDashboardPage(page);
  await this.page.locator('[data-test="dashboard-add"]').click();
  await page.locator('[data-test="add-dashboard-name"]').click();
  await this.page
    .locator('[data-test="add-dashboard-name"]')
    .fill(randomDashboardName);
  await this.page.locator('[data-test="dashboard-add-submit"]').click();

  }


}


