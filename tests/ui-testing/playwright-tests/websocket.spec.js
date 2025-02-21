import { test, expect } from "./baseFixtures";
import { LoginPage } from '../pages/loginPage';
import { LogsPage } from '../pages/logsPage.js';
import { IngestionPage } from '../pages/ingestionPage';
import { ManagementPage } from '../pages/managementPage';



test.describe.configure({ mode: 'parallel' });

test.describe("Websocket for logs", () => {
    let loginPage, logsPage, ingestionPage, managementPage;

    test.beforeEach(async ({ page }) => {
        loginPage = new LoginPage(page);
        ingestionPage = new IngestionPage(page);
        logsPage = new LogsPage(page);
        managementPage = new ManagementPage(page);
        await loginPage.gotoLoginPage();
        await loginPage.loginAsInternalUser();
        await loginPage.login(); // Login as root user
        await ingestionPage.ingestion();
        await ingestionPage.ingestionJoin();
        await managementPage.goToManagement();
        await page.waitForTimeout(5000);
        await managementPage.checkAndEnableWebSocket();
    });

    test("Run query after selecting two streams", async ({ page }) => {

        // If using CommonJS
        // const managementPage = require('../pages/managementPage');
        

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.displayTwoStreams();
        await logsPage.selectRunQuery();
     

    });

    test("Run query after selecting two streams and SQL Mode On", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.enableSQLMode();
        await logsPage.selectRunQuery();
        await logsPage.displayTwoStreams();
       

    });

    test("Run query after selecting two streams, selecting field and SQL Mode On", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.kubernetesContainerName();
        await logsPage.enableSQLMode();
        await logsPage.selectRunQuery();
        await logsPage.validateResult();
    

    });

    test("Run query after selecting two streams, SQL Mode On and entering join queries", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.kubernetesContainerNameJoin();
        await logsPage.enableSQLMode();
        await logsPage.selectRunQuery();
        await logsPage.displayCountQuery();
        await logsPage.validateResult();
       

    });

    test("Run query after selecting two streams, SQL Mode On and entering join limit", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.kubernetesContainerNameJoinLimit();
        await logsPage.enableSQLMode();
        await logsPage.selectRunQuery();
        await logsPage.validateResult();
      

    });

    test("Run query after selecting two streams, SQL Mode On and entering join like", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.kubernetesContainerNameJoinLike();
        await logsPage.enableSQLMode();
        await logsPage.selectRunQuery();
       
      

    });

    test("Run query after selecting two streams, SQL Mode On and entering Left join queries", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.kubernetesContainerNameLeftJoin();
        await logsPage.enableSQLMode();
        await logsPage.selectRunQuery();
        await logsPage.validateResult();
    

    });

    test("Run query after selecting two streams, SQL Mode On and entering Right join queries", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.kubernetesContainerNameRightJoin();
        await logsPage.enableSQLMode();
        await logsPage.selectRunQuery();
        await logsPage.validateResult();
     

    });

    test("Run query after selecting two streams, SQL Mode On and entering Full join queries", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.kubernetesContainerNameFullJoin();
        await logsPage.enableSQLMode();
        await logsPage.selectRunQuery();
        await logsPage.validateResult();
    

    });

    test("Click on interesting field icon and display field in editor", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.enableSQLMode();
        await logsPage.clickQuickModeToggle();
        await logsPage.selectRunQuery();
        await logsPage.clickInterestingFields();
        await logsPage.validateInterestingFields();
    });

    test("Click on interesting field icon and display query in editor", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.enableSQLMode();
        await logsPage.clickQuickModeToggle();
        await logsPage.selectRunQuery();
        await logsPage.clickInterestingFields();
        await logsPage.validateInterestingFieldsQuery();
    });

    test("Add/remove interesting field removes it from editor and results too", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectIndexAndStreamJoin();
        await logsPage.enableSQLMode();
        await logsPage.clickQuickModeToggle();
        await logsPage.selectRunQuery();
        await logsPage.clickInterestingFields();
        await logsPage.addRemoveInteresting();
    });

});
