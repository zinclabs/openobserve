// dashlogin.js   
export const login = async (page) => {  
    await page.goto(process.env["ZO_BASE_URL"], { waitUntil: "networkidle" });
    await page.waitForTimeout(1000);
    await page
      .locator('[data-cy="login-user-id"]')
      .fill(process.env["ZO_ROOT_USER_EMAIL"]);
  
    const waitForLogin = page.waitForResponse(
      (response) =>
        response.url().includes("/auth/login") && response.status() === 200
    );
  
    await page
      .locator('[data-cy="login-password"]')
      .fill(process.env["ZO_ROOT_USER_PASSWORD"]);
    await page.locator('[data-cy="login-sign-in"]').click();
  
    await waitForLogin;
  
    await page.waitForURL(process.env["ZO_BASE_URL"] + "/web/", {
      waitUntil: "networkidle",
    });
    await page
      .locator('[data-test="navbar-organizations-select"]')
      .getByText("arrow_drop_down")
      .click();
    await page.getByRole("option", { name: "default", exact: true }).click();
  }
  
  // module.exports = { login };
  