import { test, expect } from "./baseFixtures.js";
import { LoginPage } from '../pages/loginPage.js';
import { LogsPage } from '../pages/logsPage.js';
import { IngestionPage } from '../pages/ingestionPage.js';
import { HomePage } from "../pages/homePage.js";
import { MetricsPage } from "../pages/metricsPage.js";
import { TracesPage } from "../pages/tracesPage.js";
import { RumPage } from "../pages/rumPage.js";
import { PipelinesPage } from "../pages/pipelinesPage.js";
import { DashboardPage } from "../pages/dashboardPage.js";
import { StreamsPage } from "../pages/streamsPage.js";
import { ReportsPage } from "../pages/reportsPage.js";
import { AlertsPage } from "../pages/alertsPage.js";
import { DataPage } from "../pages/dataPage.js";
import { IamPage } from "../pages/iamPage.js";
import { ManagementPage } from "../pages/managementPage.js";
import { AboutPage } from "../pages/aboutPage.js";


test.describe("Change Organisation", () => {
    let loginPage, logsPage, ingestionPage, homePage, metricsPage,
        tracesPage, rumPage, pipelinesPage, dashboardPage, streamsPage, 
        reportsPage, alertsPage, dataPage, iamPage, managementPage, aboutPage, defaulttestmulti;

    test.beforeEach(async ({ page }) => {
        loginPage = new LoginPage(page);
        ingestionPage = new IngestionPage(page);
        logsPage = new LogsPage(page);
        homePage = new HomePage(page);
        metricsPage = new MetricsPage(page);
        tracesPage = new TracesPage(page);
        rumPage = new RumPage(page);
        pipelinesPage = new PipelinesPage(page);
        dashboardPage = new DashboardPage(page);
        streamsPage = new StreamsPage(page);
        reportsPage = new ReportsPage(page);
        alertsPage = new AlertsPage(page);
        dataPage = new DataPage(page);
        iamPage = new IamPage(page);
        managementPage = new ManagementPage(page);
        aboutPage = new AboutPage(page);
        await loginPage.gotoLoginPage();
        await loginPage.loginAsInternalUser();
        await loginPage.login();
        await ingestionPage.ingestion();
        await ingestionPage.ingestionMultiOrg();
    });

    test("Home Page default validation", async ({ page }) => {

        await homePage.homePageValidation();
        await homePage.gotoHomePage();
        await homePage.homePageValidation();


    });

    test("Home Page change organisation validation", async ({ page }) => {

        await homePage.homePageDefaultMultiOrg();
        await homePage.homePageValidation();
        await homePage.homePageURLValidation();
        await homePage.gotoHomePage();
        await homePage.homePageValidation();
        await homePage.homePageURLValidation();

    });

    test("Logs Page default validation", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.validateLogsPage();
    });

    test("Logs Page change organisation validation", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.logsPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await logsPage.validateLogsPage();

        await logsPage.logsPageURLValidation();
        await logsPage.navigateToLogs();
        await logsPage.validateLogsPage();
        await logsPage.logsPageURLValidation();

    });

    test("Metrics Page default validation", async ({ page }) => {


        await metricsPage.gotoMetricsPage();
        await metricsPage.metricsPageValidation();
        await metricsPage.metricsURLValidation();
    });

    test("Metrics Page change organisation validation", async ({ page }) => {

        await metricsPage.gotoMetricsPage();
        await page.waitForTimeout(5000);
        await metricsPage.metricsPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await metricsPage.metricsPageValidation();

        await metricsPage.metricsPageURLValidation();
        await metricsPage.gotoMetricsPage();
        await metricsPage.metricsPageValidation();
        await metricsPage.metricsPageURLValidation();

    });

    test("Traces Page default validation", async ({ page }) => {


        await tracesPage.navigateToTraces();
        await tracesPage.validateTracesPage();
        await tracesPage.tracesURLValidation();
    });

    test("Traces Page change organisation validation", async ({ page }) => {

        await tracesPage.navigateToTraces();
        await page.waitForTimeout(5000);
        await tracesPage.tracesPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await tracesPage.validateTracesPage();

        await tracesPage.tracesPageURLValidation();
        await tracesPage.navigateToTraces();
        await tracesPage.validateTracesPage();
        await tracesPage.tracesPageURLValidation();

    });

    test("RUM Page default validation", async ({ page }) => {


        await rumPage.gotoRumPage();
        await rumPage.rumURLValidation();
    });

    test("RUM Page change organisation validation", async ({ page }) => {

        await rumPage.gotoRumPage();
        await page.waitForTimeout(5000);
        await rumPage.rumPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await rumPage.rumPageURLValidation();
        await rumPage.gotoRumPage();
        await rumPage.rumPageURLValidation();

    });


    test("Pipelines Page default validation", async ({ page }) => {


        await pipelinesPage.gotoPipelinesPage();
        await pipelinesPage.pipelinesURLValidation();
    });

    test("Pipelines Page change organisation validation", async ({ page }) => {

        await pipelinesPage.gotoPipelinesPage();
        await page.waitForTimeout(5000);
        await pipelinesPage.pipelinesPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await pipelinesPage.pipelinesPageURLValidation();
        await pipelinesPage.gotoPipelinesPage();
        await pipelinesPage.pipelinesPageURLValidation();

    });

    test("Dashboard Page default validation", async ({ page }) => {


        await dashboardPage.navigateToDashboards();
        await dashboardPage.dashboardURLValidation();
    });

    test("Dashboard Page change organisation validation", async ({ page }) => {

        await dashboardPage.navigateToDashboards();
        await page.waitForTimeout(5000);
        await dashboardPage.dashboardPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await dashboardPage.dashboardPageURLValidation();
        await dashboardPage.navigateToDashboards();
        await dashboardPage.dashboardPageURLValidation();

    });

    test("Streams Page default validation", async ({ page }) => {


        await streamsPage.gotoStreamsPage();
        await streamsPage.streamsURLValidation();
    });

    test("Streams Page change organisation validation", async ({ page }) => {

        await streamsPage.gotoStreamsPage();
        await page.waitForTimeout(5000);
        await streamsPage.streamsPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await streamsPage.streamsPageURLValidation();
        await streamsPage.gotoStreamsPage();
        await streamsPage.streamsPageURLValidation();

    });

    test("Reports Page default validation", async ({ page }) => {


        await reportsPage.goToReports();
        await reportsPage.reportsURLValidation();
    });

    test("Reports Page change organisation validation", async ({ page }) => {

        await reportsPage.goToReports();
        await page.waitForTimeout(5000);
        await reportsPage.reportsPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await reportsPage.reportsPageURLValidation();
        await reportsPage.goToReports();
        await reportsPage.reportsPageURLValidation();

    });

    test("Alerts Page default validation", async ({ page }) => {


        await alertsPage.navigateToAlerts();
        await alertsPage.alertsURLValidation();
    });

    test("Alerts Page change organisation validation", async ({ page }) => {

        await alertsPage.navigateToAlerts();
        await page.waitForTimeout(5000);
        await alertsPage.alertsPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await alertsPage.alertsPageURLValidation();
        await alertsPage.navigateToAlerts();
        await alertsPage.alertsPageURLValidation();

    });

    test("Data sources Page default validation", async ({ page }) => {


        await dataPage.gotoDataPage();
        await dataPage.dataURLValidation();
    });

    test("Data sources Page change organisation validation", async ({ page }) => {

        await dataPage.gotoDataPage();
        await page.waitForTimeout(5000);
        await dataPage.dataPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await dataPage.dataPageURLValidation();
        await dataPage.gotoDataPage();
        await dataPage.dataPageURLValidation();

    });

    test("IAM Page default validation", async ({ page }) => {


        await iamPage.gotoIamPage();
        await iamPage.iamURLValidation();
    });

    test("IAM Page change organisation validation", async ({ page }) => {

        await iamPage.gotoIamPage();
        await page.waitForTimeout(5000);
        await iamPage.iamPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await iamPage.iamPageURLValidation();
        await iamPage.gotoIamPage();
        await iamPage.iamPageURLValidation();

    });

    test("Management Page default validation", async ({ page }) => {


        await managementPage.goToManagement();
        await managementPage.managementURLValidation();
    });

    test("Management Page change organisation validation", async ({ page }) => {

        await managementPage.goToManagement();
        await page.waitForTimeout(5000);
        await managementPage.managementPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await managementPage.managementPageURLValidation();
        await managementPage.goToManagement();
        await managementPage.managementPageURLValidation();

    });

    test("About Page default validation", async ({ page }) => {

        await aboutPage.clickHelpMenu();
        await aboutPage.gotoAboutPage();
        await aboutPage.aboutURLValidation();
    });

    test("About Page change organisation validation", async ({ page }) => {
        await aboutPage.clickHelpMenu();
        await aboutPage.gotoAboutPage();
        await page.waitForTimeout(5000);
       await aboutPage.aboutPageDefaultMultiOrg();
        await page.waitForTimeout(5000);
        await aboutPage.aboutPageURLValidation();
        await aboutPage.clickHelpMenu();
        await aboutPage.gotoAboutPage();
        await aboutPage.aboutURLValidation();

    });

   

});
