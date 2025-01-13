import { expect } from '@playwright/test';
import {
  dateTimeButtonLocator, relative30SecondsButtonLocator, absoluteTabLocator,
  Past30SecondsValue
} from '../pages/CommonLocator.js';

export class ReportsPage {
  constructor(page) {
    this.page = page;
    this.homeMenu = page.locator("[name ='home']");
    this.reportsMenu = page.locator('[data-test="menu-link-\\/reports-item"]');
    this.scheduledTab = page.locator("[title='Scheduled']");
    this.addReportButton = page.locator('[data-test="report-list-add-report-btn"]');
    this.reportNameInput = page.locator("[aria-label='Name *']");
    this.folderInput = page.locator("[aria-label='Folder *']");
    this.dashboardInput = page.locator("[aria-label='Dashboard *']");
    this.dashboardTabInput = page.locator("[aria-label='Dashboard Tab *']");
    this.continueButtonStep1 = page.locator('[data-test="add-report-step1-continue-btn"]');
    this.continueButtonStep2 = page.locator('[data-test="add-report-step2-continue-btn"]');
    this.titleInput = page.locator("[aria-label='Title *']");
    this.recipientsInput = page.locator("[aria-label='Recipients *']");
    this.saveButton = page.locator('[data-test="add-report-save-btn"]');
    this.successAlert = page.getByRole('alert').nth(1);
    this.dateTimeButton = dateTimeButtonLocator;
    this.relative30SecondsButton = page.locator(relative30SecondsButtonLocator);
    this.absoluteTab = absoluteTabLocator;
    this.profileButton = page.locator('button').filter({ hasText: (process.env["ZO_ROOT_USER_EMAIL"]) });
    this.zoneInput = page.locator('[data-test="add-report-schedule-send-later-section"]').getByText('arrow_drop_down');
    this.timeZoneOption = (zone) => `role=option[name="${zone}"]`;
    this.signOutButton = page.getByText('Sign Out');
  }

  async navigateToReports() {
    await this.homeMenu.hover();
    await this.reportsMenu.click({ force: true });
    await expect(this.page.locator('[data-test="report-list-title"]')).toContainText('Reports');
    await this.scheduledTab.click({ force: true });
  }

  async goToReports() {

    await this.reportsMenu.click({ force: true });
    await expect(this.page.locator('[data-test="report-list-title"]')).toContainText('Reports');

  }

  async reportsPageDefaultMultiOrg() {
    await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click({ force: true });
    await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click({ force: true });
  }

  async reportsPageURLValidation() {
    // TODO: fix this test
    // await expect(this.page).not.toHaveURL(/default/);
  }

  async reportsURLValidation() {
    await expect(this.page).toHaveURL(/report/);
  }





  async createReportAddReportButton() {
    await this.page.waitForSelector('[data-test="report-list-add-report-btn"]');
    await this.addReportButton.click({ force: true });
  }

  async createReportReportNameInput() {
    await this.page.waitForSelector("[aria-label='Name *']");
    await this.reportNameInput.fill("rreport1");
    await this.page.waitForTimeout(5000);
  }

  async createReportFolderInput() {
    await this.folderInput.dblclick({ force: true });
    await this.page.waitForLoadState("networkidle");
    await this.folderInput.pressSequentially('de', { delay: 100 });
    await this.page.getByRole('option', { name: 'default' }).click({ force: true });
  }

  async createReportDashboardInput(dashboardName) {
    await this.dashboardInput.dblclick({ force: true });
    await this.page.waitForLoadState("networkidle");
    await this.dashboardInput.pressSequentially(dashboardName, { delay: 100 });
    await this.page.getByRole('option', { name: dashboardName }).click({ force: true });
  }

  async createReportDashboardTabInput() {
    await this.dashboardTabInput.dblclick({ force: true });
    await this.page.waitForLoadState("networkidle");
    await this.dashboardTabInput.pressSequentially('de', { delay: 100 });
    await this.page.getByRole('option', { name: 'default' }).click({ force: true });
  }

  async createReportContinueButtonStep1() {
    await this.continueButtonStep1.click({ force: true });
    await this.page.waitForTimeout(3000);
  }

  async createReportOnce() {
    await this.page.locator('[data-test="add-report-schedule-frequency-once-btn"]').click({ force: true });

  }

  async createReportHours() {
    await this.page.locator('[data-test="add-report-schedule-frequency-hours-btn"]').click({ force: true });

  }

  async createReportDays() {
    await this.page.locator('[data-test="add-report-schedule-frequency-days-btn"]').click({ force: true });

  }

  async createReportWeeks() {
    await this.page.locator('[data-test="add-report-schedule-frequency-weeks-btn"]').click({ force: true });

  }

  async createReportMonths() {
    await this.page.locator('[data-test="add-report-schedule-frequency-months-btn"]').click({ force: true });

  }

  async createReportCustom() {
    await this.page.locator('[data-test="add-report-schedule-frequency-custom-btn"]').click({ force: true });

  }

  async createReportCron() {
    await this.page.locator('[data-test="add-report-schedule-frequency-cron-btn"]').click({ force: true });

  }

  async createReportDateTime() {

    await this.page.getByLabel('Start Date *').fill('29-12-2025');
    await this.page.getByLabel('Start Time *').fill('11:55');
  }

  async createReportZone() {

    await this.zoneInput.dblclick({ force: true });
    await this.page.waitForLoadState("networkidle");
    await this.zoneInput.pressSequentially('UTC', { delay: 1000 });
    await this.page.getByRole('option', { name: 'UTC', exact: true }).waitFor({ state: 'visible' });
    await this.page.getByRole('option', { name: 'UTC', exact: true }).click();
    await this.page.waitForTimeout(5000);
  }

  async setTimeZone(zone) {
    await this.zoneInput.click({ force: true });
    await this.page.getByRole(this.timeZoneOption(zone)).click({ force: true });
  }

  async setTimeIST() {
    await this.zoneInput.fill("Asia/c");
    await page.getByText("Asia/Calcutta", { exact: true }).click({ force: true });

  }

  async createReportScheduleLater() {

    await this.page.locator('[data-test="add-report-schedule-scheduleLater-btn"]').click({ force: true });

  }

  async createReportContinueButtonStep2() {
    await this.continueButtonStep2.click({ force: true });
  }

  async createReportFillDetail() {
    await this.titleInput.fill("reporterTest");
    await this.recipientsInput.fill(process.env["ZO_ROOT_USER_EMAIL"]);
  }

  async createReportSaveButton() {
    await this.saveButton.click({ force: true });
  }

  async createReport(dashboardName) {
    await this.page.waitForSelector('[data-test="report-list-add-report-btn"]');
    await this.addReportButton.click({ force: true });
    await this.page.waitForSelector("[aria-label='Name *']");
    await this.reportNameInput.fill("rreport1");
    await this.page.waitForTimeout(5000);
    await this.folderInput.dblclick({ force: true });
    await this.page.waitForLoadState("networkidle");
    await this.folderInput.pressSequentially('de', { delay: 100 });
    await this.page.getByRole('option', { name: 'default' }).click({ force: true });
    await this.dashboardInput.dblclick({ force: true });
    await this.page.waitForLoadState("networkidle");
    await this.dashboardInput.pressSequentially(dashboardName, { delay: 100 });
    await this.page.getByRole('option', { name: dashboardName }).click({ force: true });
    await this.dashboardTabInput.dblclick({ force: true });
    await this.page.waitForLoadState("networkidle");
    await this.dashboardTabInput.pressSequentially('de', { delay: 100 });
    await this.page.getByRole('option', { name: 'default' }).click({ force: true });
    await this.continueButtonStep1.click({ force: true });
    await this.continueButtonStep2.click({ force: true });
    await this.titleInput.fill("reporterTest");
    await this.recipientsInput.fill(process.env["ZO_ROOT_USER_EMAIL"]);
    await this.saveButton.click({ force: true });
  }
  async verifyReportSaved() {
    await expect(this.successAlert).toContainText('Report saved successfully.');
  }
  async deleteReport(reportName) {
    await this.page.locator('[data-test="report-list-search-input"]').fill(reportName);
    await this.page
      .locator(`[data-test="report-list-${reportName}-delete-report"]`)
      .click({ force: true });
    await this.page.locator('[data-test="confirm-button"]', 'visible').click();
  }
  async setTimeToPast30Seconds() {
    // Set the time filter to the last 30 seconds
    await this.page.locator(this.dateTimeButton).click({ force: true });
    await this.relative30SecondsButton.click({ force: true });
  }
  async verifyTimeSetTo30Seconds() {
    // Verify that the time filter displays "Past 30 Seconds"
    await expect(this.page.locator(this.dateTimeButton)).toContainText(Past30SecondsValue);
  }
  async setDateTime() {
    await expect(this.page.locator(this.dateTimeButton)).toBeVisible();
    await this.page.locator(this.dateTimeButton).click({ force: true });
    await this.page.locator(this.absoluteTab).click({ force: true });
    await this.page.waitForTimeout(1000);

  }

  async fillTimeRange(startTime, endTime) {
    await this.page.getByRole('button', { name: '1', exact: true }).click({ force: true });
    await this.page.getByLabel('access_time').first().fill(startTime);
    await this.page.getByRole('button', { name: '1', exact: true }).click({ force: true });
    await this.page.getByLabel('access_time').nth(1).fill(endTime);

  }

  async verifyDateTime(startTime, endTime) {
    await expect(this.page.locator(this.dateTimeButton)).toContainText(`${startTime} - ${endTime}`);
  }

  async signOut() {
    await this.profileButton.click({ force: true });
    await this.signOutButton.click({ force: true });
  }
}


