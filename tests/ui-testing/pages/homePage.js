// homePage.js
import { expect } from '@playwright/test';
//import { validateUrlContains } from './utils/validateUrlContains';

export class HomePage {
    constructor(page) {
        this.page = page;
        this.homePageMenu = page.locator('[data-test="menu-link-\\/-item"]');

    }
    async gotoHomePage() {

        await this.homePageMenu.click();

    }

    async homePageValidation() {

        await expect(this.page.getByRole('main')).toContainText('Streams');
        await expect(this.page.getByRole('main')).toContainText('Function');
        await expect(this.page.getByRole('main')).toContainText('Scheduled');


    }


    async homePageDefaultOrg() {
        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByText('default', { exact: true }).click();
    }

    async homePageDefaultMultiOrg() {
        // await this.page.pause();
        await this.page.waitForTimeout(5000);
        await this.page.reload();
        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.waitForTimeout(5000);

        await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();
    }

    async homePageURLValidation() {
        // TODO: Fix this test
        await expect(this.page).not.toHaveURL(/default/);
    }
}