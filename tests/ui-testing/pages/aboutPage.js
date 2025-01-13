// aboutPage.js
import { expect } from '@playwright/test';


export class AboutPage {
    constructor(page) {
        this.page = page;
        this.aboutPageMenu = page.locator('[data-test="menu-link-\\/about-item"]');

    }
    async gotoAboutPage() {

        await this.aboutPageMenu.click();

    }


    async aboutPageDefaultOrg() {

        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByText('default', { exact: true }).click();


    }

    async aboutPageDefaultMultiOrg() {
        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();
    }

    async aboutPageURLValidation() {
        // TODO: Fix this test
        // await expect(this.page).not.toHaveURL(/default/);
    }

    async aboutURLValidation() {
        await expect(this.page).toHaveURL(/about/);
    }

    async selectOrganization(orgName) {
        const dropdown = this.page.locator('[data-test="navbar-organizations-select"]');
        await dropdown.getByText('arrow_drop_down').click();

        try {
            if (orgName === 'default') {
                await this.page.getByText('default', { exact: true }).click();
            } else {
                await this.page.getByRole('option', { name: orgName }).locator('div').nth(2).click();
            }
            await this.page.waitForLoadState('networkidle');
        } catch (error) {
            throw new Error(`Failed to select organization: ${orgName}`);
        }
    }


}
