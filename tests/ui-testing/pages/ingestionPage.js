
import logsdata from "../../test-data/logs_data.json";
export class IngestionPage {
  constructor(page) {
    this.page = page;
  }
  async ingestion() {
    const orgId = process.env["ORGNAME"];
    const streamName = "e2e_automate";
    const basicAuthCredentials = Buffer.from(`${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`).toString('base64');
    const headers = {
      "Authorization": `Basic ${basicAuthCredentials}`,
      "Content-Type": "application/json",
    };
    const fetchResponse = await fetch(
      `${process.env.INGESTION_URL}/api/${orgId}/${streamName}/_json`,
      {
        method: "POST",
        headers: headers,
        body: JSON.stringify(logsdata),
      }
    );
    try {
      const response = await fetchResponse;
      const contentType = response.headers.get("content-type");
      if (contentType && contentType.includes("application/json")) {
        const jsonData = await response.json();
        console.log(response.status);
        console.log(response.url);
        // Process JSON data here
      } else {
        const textData = await response.text();
        console.log("Response is not JSON:", textData);
        // Handle the non-JSON response here
      }
    } catch (error) {
      console.error("Failed to parse JSON response:", error);
    }
  }

  async ingestionJoin() {
    const orgId = process.env["ORGNAME"];
    const streamName = "default";
    const basicAuthCredentials = Buffer.from(`${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`).toString('base64');
    const headers = {
      "Authorization": `Basic ${basicAuthCredentials}`,
      "Content-Type": "application/json",
    };
    const fetchResponse = await fetch(
      `${process.env.INGESTION_URL}/api/${orgId}/${streamName}/_json`,
      {
        method: "POST",
        headers: headers,
        body: JSON.stringify(logsdata),
      }
    );
    try {
      const response = await fetchResponse;
      const contentType = response.headers.get("content-type");
      if (contentType && contentType.includes("application/json")) {
        const jsonData = await response.json();
        console.log(response.status);
        console.log(response.url);
        // Process JSON data here
      } else {
        const textData = await response.text();
        console.log("Response is not JSON:", textData);
        // Handle the non-JSON response here
      }
    } catch (error) {
      console.error("Failed to parse JSON response:", error);
    }
  }

  async ingestionMultiOrg(orgId) {
    const streamName = "e2e_automate";
    const basicAuthCredentials = Buffer.from(`${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`).toString('base64');
    const headers = {
      "Authorization": `Basic ${basicAuthCredentials}`,
      "Content-Type": "application/json",
    };
    const fetchResponse = await fetch(
      `${process.env.INGESTION_URL}/api/${orgId}/${streamName}/_json`,
      {
        method: "POST",
        headers: headers,
        body: JSON.stringify(logsdata),
      }
    );
    try {
      const response = await fetchResponse;
      const contentType = response.headers.get("content-type");
      if (contentType && contentType.includes("application/json")) {
        const jsonData = await response.json();
        console.log(response.status);
        console.log(response.url);
        // Process JSON data here
      } else {
        const textData = await response.text();
        console.log("Response is not JSON:", textData);
        // Handle the non-JSON response here
      }
    } catch (error) {
      console.error("Failed to parse JSON response:", error);
    }
  }
}
