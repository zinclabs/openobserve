// tracesPage.js
import { expect } from '@playwright/test';

import { dateTimeButtonLocator, relative30SecondsButtonLocator, absoluteTabLocator, Past30SecondsValue } from '../pages/CommonLocator.js';


export
  class TracesPage {
  constructor(page) {
    this.page = page;
    this.tracesMenuItem = page.locator('[data-test="menu-link-\\/traces-item"]');

    this.dateTimeButton = dateTimeButtonLocator;
    this.relative30SecondsButton = page.locator(relative30SecondsButtonLocator);
    this.absoluteTab = absoluteTabLocator;


    this.profileButton = page.locator('button').filter({ hasText: (process.env["ZO_ROOT_USER_EMAIL"]) });
    this.signOutButton = page.getByText('Sign Out');


  }

  async navigateToTraces() {

    await this.tracesMenuItem.click();
  }

  async validateTracesPage() {

    await expect(this.page.locator('[data-test="logs-search-bar-sql-mode-toggle-btn"]')).toContainText('Syntax Guide');
    await expect(this.page.locator('[data-test="logs-search-bar"]')).toContainText('Reset Filters');

  }

  async tracesPageDefaultOrg() {

    await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
    await this.page.getByText('default', { exact: true }).click();


  }

  async tracesPageDefaultMultiOrg() {



    await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
    // await this.page.pause();
    // await this.page.waitForTimeout(5000);

    await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();


    // await validateUrlContains(this.page, 'path');


  }

  async tracesPageURLValidation() {
    // TODO: fix the test
    // await expect(this.page).not.toHaveURL(/default/);

  }

  async tracesURLValidation() {

    await expect(this.page).toHaveURL(/traces/);

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
    await this.page.waitForTimeout(1000);

  }

  async fillTimeRange(startTime, endTime) {
    await this.page.getByRole('button', { name: '1', exact: true }).click();
    await this.page.getByLabel('access_time').first().fill(startTime);
    await this.page.getByRole('button', { name: '1', exact: true }).click();
    await this.page.getByLabel('access_time').nth(1).fill(endTime);
    // await this.page.waitForTimeout(1000);
  }

  async verifyDateTime(startTime, endTime) {
    await expect(this.page.locator(this.dateTimeButton)).toContainText(`${startTime} - ${endTime}`);
  }

  async signOut() {
    await this.profileButton.click();
    await this.signOutButton.click();
  }

}


