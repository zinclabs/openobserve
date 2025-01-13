// pipelinesPage.js
import { expect } from '@playwright/test';


export class PipelinesPage {
    constructor(page) {
        this.page = page;
        this.pipelinesPageMenu = page.locator('[data-test="menu-link-\\/pipeline-item"]');

    }

    async gotoPipelinesPage() {
        await this.pipelinesPageMenu.click();
    }


    async pipelinesPageDefaultOrg() {

        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByText('default', { exact: true }).click();


    }

    async pipelinesPageDefaultMultiOrg() {
        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();
    }

    async pipelinesPageURLValidation() {
        //TODO Fix this test
        // await expect(this.page).not.toHaveURL(/default/);
    }

    async pipelinesURLValidation() {
        await expect(this.page).toHaveURL(/pipeline/);
    }

}