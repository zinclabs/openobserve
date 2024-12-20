const { expect } = require('@playwright/test')

export class DashFilter {
    constructor(page) {
      this.page = page;
  
      // Locators
      this.dashboardMenuLink = page.locator('[data-test="menu-link-\\/dashboards-item"]');
      this.addDashboardButton = page.locator('[data-test="dashboard-add"]');
      this.dashboardNameField = page.locator('[data-test="add-dashboard-name"]');
      this.submitDashboardButton = page.locator('[data-test="dashboard-add-submit"]');


      this.settingsButton = page.locator('[data-test="dashboard-setting-btn"]');

      
      this.variablesTab = page.getByRole("tab", { name: "Variables" });
      this.addVariableButton = page.getByRole("button", { name: "Add Variable" });
      this.variableNameField = page.getByLabel("Name *");
      this.streamTypeDropdown = page.locator("label").filter({ hasText: "Stream Type *arrow_drop_down" }).locator("i");
      this.streamDropdown = page.locator("label").filter({ hasText: "Stream *arrow_drop_down" }).locator("i");
      this.streamOption = page.getByRole("option", { name: "e2e_automate" });
      this.fieldDropdown = page.getByText("Field *arrow_drop_down");
      this.saveVariableButton = page.getByRole("button", { name: "Save" });
      this.addPanelButton = page.locator('[data-test="dashboard-if-no-panel-add-panel-btn"]');
      this.streamSelector = page.locator('[data-test="index-dropdown-stream"]');
      this.applyButton = page.locator('[data-test="dashboard-apply"]');
      this.queryInspectorButton = page.locator('[data-test="dashboard-panel-data-view-query-inspector-btn"]');
      this.closeQueryInspectorButton = page.locator('[data-test="query-inspector-close-btn"]');
      this.panelNameField = page.locator('[data-test="dashboard-panel-name"]');
      this.panelSaveButton = page.locator('[data-test="dashboard-panel-save"]');
    }
  
    // Methods
    async openDashboardMenu() {
      await this.dashboardMenuLink.click();
    }
  
    async addDashboard(dashboardName) {
      await this.addDashboardButton.click();
      await this.dashboardNameField.fill(dashboardName);
      await this.submitDashboardButton.click();
    }
  
    async configureVariable(variableName, streamType = "logs", stream = "e2e_automate", field = "kubernetes_container_name") {
      await this.settingsButton.click();
      await this.variablesTab.click();
      await this.addVariableButton.click();
      await this.variableNameField.fill(variableName);    
  
      await this.streamTypeDropdown.click();
      await this.page.getByRole("option", { name: streamType }).click();
  
      await this.streamDropdown.click();
      await this.streamOption.click();
  
      await this.fieldDropdown.click();
      await this.page.getByRole("option", { name: field }).click();
  
      await this.saveVariableButton.click();
    }
  
    async addPanel() {
      await this.addPanelButton.click();
    }
  
    async applyStreamandField (stream = "e2e_automate", field = "_timestamp") {
      await this.streamSelector.click();
      await this.page.getByRole("option", { name: stream }).click();
      await this.page.locator(`[data-test="field-list-item-logs-${stream}-${field}"] [data-test="dashboard-add-y-data"]`).click();
      await this.applyButton.click();
    }
  
    async verifyQueryInspector(query) {
      await this.queryInspectorButton.click();
      await this.page.getByRole("cell", { name: query, exact: true }).waitFor();
      await this.closeQueryInspectorButton.click();
    }
  
    async savePanel(name) {
      await this.panelNameField.fill(name);
      await this.panelSaveButton.click();
    }


//     // Method to interact with a filter button by its identifier
//   async clickFilterButton(filterIdentifier: string) {
//     const filterButton = this.page.locator(
//       `[data-test="field-list-item-logs-e2e_automate-${filterIdentifier}"] [data-test="dashboard-add-filter-data"]`
//     );
//     await expect(filterButton).toBeVisible();
//     await filterButton.click();
//   }

  }
  
  
  