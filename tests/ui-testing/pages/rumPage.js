// rumPage.js
import { expect } from '@playwright/test';


export class RumPage {
    constructor(page) {
        this.page = page;
        this.rumPageMenu = page.locator('[data-test="menu-link-\\/rum\\/performance\\/overview-item"]');

    }
    async gotoRumPage() {

        await this.rumPageMenu.click();

    }


    async rumPageDefaultOrg() {

        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByText('default', { exact: true }).click();


    }

    async rumPageDefaultMultiOrg() {



        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();


        await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();




    }

    async rumPageURLValidation() {

        // TODO: Fix this test
        // await expect(this.page).not.toHaveURL(/default/);

    }

    async rumURLValidation() {

        await expect(this.page).toHaveURL(/rum/);

    }

}