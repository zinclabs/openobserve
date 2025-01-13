import { expect } from '@playwright/test';

import { dateTimeButtonLocator, relative30SecondsButtonLocator, absoluteTabLocator } from '../pages/CommonLocator.js';

export class AlertsPage {
  constructor(page) {
    this.page = page;
    this.alertMenu = this.page.locator('[data-test="menu-link-\\/alerts-item"]');
    this.addAlertButton = this.page.locator('[data-test="alert-list-add-alert-btn"]');
    this.sqlOption = this.page.getByText('SQL');
    this.addTimeRangeButton = this.page.locator('[data-test="multi-time-range-alerts-add-btn"]');


    this.dateTimeButton = dateTimeButtonLocator;
    this.relative30SecondsButton = page.locator(relative30SecondsButtonLocator);
    this.absoluteTab = absoluteTabLocator;



    this.profileButton = page.locator('button').filter({ hasText: (process.env["ZO_ROOT_USER_EMAIL"]) });
    this.signOutButton = page.getByText('Sign Out');

  }

  async navigateToAlerts() {
    await this.alertMenu.click();
    await expect(this.page.locator('[data-test="alerts-list-title"]')).toContainText('Alerts');
  }

  async alertsPageDefaultMultiOrg() {
    await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
    await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();
  }

  async alertsPageURLValidation() {
    // TODO: Make sure the url contains the id of the selected org
    // await expect(this.page).not.toHaveURL(/default/);
  }

  async alertsURLValidation() {
    await expect(this.page).toHaveURL(/alerts/);
  }

  async createAlerts() {
    await this.addAlertButton.click();
    await this.sqlOption.click();
    await this.addTimeRangeButton.click();

  }

  async setTimeToPast30Seconds() {
    // Set the time filter to the last 30 seconds
    await this.page.locator(this.dateTimeButton).click();
    await this.relative30SecondsButton.click();
  }

  async verifyTimeSetTo30Seconds() {
    // Verify that the time filter displays "Past 30 Seconds"
    // await expect(this.page.locator(this.dateTimeButton)).toContainText(process.env["Past30SecondsValue"]);
    await expect(this.page.locator(this.dateTimeButton)).toContainText('schedule30 Seconds agoarrow_drop_down');
  }


  async signOut() {
    await this.profileButton.click();
    await this.signOutButton.click();
  }

}