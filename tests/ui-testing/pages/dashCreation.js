// fuction of Dashboard page and Apply quarybutton
export const waitForDashboardPage = async function (page) {
  const dashboardListApi = page.waitForResponse(
    (response) =>
      /\/api\/.+\/dashboards/.test(response.url()) && response.status() === 200
  );

  await page.waitForURL(process.env["ZO_BASE_URL"] + "/web/dashboards**");

  await page.waitForSelector(`text="Please wait while loading dashboards..."`, {
    state: "hidden",
  });
  await dashboardListApi;
  await page.waitForTimeout(500);
};

export const applyQueryButton = async function (page) {
  const search = page.waitForResponse(logData.applyQuery);
  await page.waitForTimeout(3000);
  await page.locator("[data-test='logs-search-bar-refresh-btn']").click({
    force: true,
  });
  await expect.poll(async () => (await search).status()).toBe(200);
};
