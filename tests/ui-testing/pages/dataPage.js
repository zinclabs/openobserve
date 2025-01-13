// dataPage.js
import { expect } from '@playwright/test';


export class DataPage {
    constructor(page) {
        this.page = page;
        this.dataPageMenu = page.locator('[data-test="menu-link-\\/ingestion-item"]');

    }

    async gotoDataPage() {
        await this.dataPageMenu.click();
        await expect(this.page.getByRole('main')).toContainText('Data sources');
    }


    async dataPageDefaultOrg() {

        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByText('default', { exact: true }).click();


    }

    async dataPageDefaultMultiOrg() {
        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();
    }

    async dataPageURLValidation() {
        // TODO: Fix this test
        // await expect(this.page).not.toHaveURL(/default/);
    }

    async dataURLValidation() {
        await expect(this.page).toHaveURL(/ingestion/);
    }

}