// ingestion.js
import logsdata from "../../test-data/logs_data.json"

// Exported function to remove UTF characters
export const removeUTFCharacters = (text) => {
     // console.log(text, "tex");
    // Remove UTF characters using regular expression
  return text.replace(/[^\x00-\x7F]/g, " ");
};


// Exported ingestion function
 export const ingestion = async (page)=> {
    const orgId = process.env["ORGNAME"];
    const streamName = "e2e_automate";
    const basicAuthCredentials = Buffer.from(
      `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
    ).toString("base64");
  
    const headers = {
      Authorization: `Basic ${basicAuthCredentials}`,
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
    const response = await fetchResponse.json();
    console.log(response);
  }
  
  // module.exports = { ingestion };
  