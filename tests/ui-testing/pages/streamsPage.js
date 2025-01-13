// streamPage.js
import { expect } from '@playwright/test';


export class StreamsPage {
    constructor(page) {
        this.page = page;
        this.streamsPageMenu = page.locator('[data-test="menu-link-\\/streams-item"]');

    }
    async gotoStreamsPage() {

        await this.streamsPageMenu.click();

    }


    async streamsPageDefaultOrg() {

        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByText('default', { exact: true }).click();


    }

    async streamsPageDefaultMultiOrg() {



        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();


        await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();




    }

    async streamsPageURLValidation() {
        // TODO: Fix this test
        // await expect(this.page).not.toHaveURL(/default/);

    }

    async streamsURLValidation() {

        await expect(this.page).toHaveURL(/streams/);

    }

}
