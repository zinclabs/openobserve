// metricsPage.js
import { expect } from '@playwright/test';


export class MetricsPage {
    constructor(page) {
        this.page = page;
        this.metricsPageMenu = page.locator('[data-test="menu-link-\\/metrics-item"]');

    }
    async gotoMetricsPage() {

        await this.metricsPageMenu.click();

    }

    async metricsPageValidation() {

        await expect(this.page.locator('[data-cy="syntax-guide-button"]')).toContainText('Syntax Guide');
        //await expect(this.page.locator('[data-test="zinc-metrics"]')).toContainText('PromQL:');


    }


    async metricsPageDefaultOrg() {

        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByText('default', { exact: true }).click();


    }

    async metricsPageDefaultMultiOrg() {



        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        // await this.page.pause();
        // await this.page.waitForTimeout(5000);

        await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();


        // await validateUrlContains(this.page, 'path');


    }

    async metricsPageURLValidation() {
        // TODO: fix the test
        // await expect(this.page).not.toHaveURL(/default/);

    }

    async metricsURLValidation() {

        await expect(this.page).toHaveURL(/metrics/);

    }

}