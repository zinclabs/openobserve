
export class CreateOrgPage {
    constructor(page) {
        this.page = page;
    }
    async createOrg() {
        const basicAuthCredentials = Buffer.from(`${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`).toString('base64');
        const headers = {
            "Authorization": `Basic ${basicAuthCredentials}`,
            "Content-Type": "application/json",
        };

        let newOrgBody = {
            "name": "defaulttestmulti",
        }

        const fetchResponse = await fetch(
            `${process.env.INGESTION_URL}/api/organizations`,
            {
                method: "POST",
                headers: headers,
                body: JSON.stringify(newOrgBody),
            }
        );
        try {
            const response = await fetchResponse;
            const contentType = response.headers.get("content-type");
            if (contentType && contentType.includes("application/json")) {
                const jsonData = await response.json();
                console.log(response.status);
                console.log(jsonData);
                return jsonData.identifier;
                // Process JSON data here
            } else {
                const textData = await response.text();
                console.log("Response is not JSON:", textData);
                return textData;
                // Handle the non-JSON response here
            }
        } catch (error) {
            console.error("Failed to parse JSON response:", error);
            return "";
        }
    }
}
