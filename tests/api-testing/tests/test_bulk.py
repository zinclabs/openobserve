import json

def test_e2e_bulk_ingest(create_session, base_url):
    """
    Running an E2E test for bulk ingestion using the _bulk API.
    This includes both positive (valid payload) and negative (invalid payload) cases.
    """
    session = create_session
    org_id = "default"
    url = f"{base_url}api/{org_id}/_bulk"
    headers = {"Content-Type": "application/json"}

    # **Positive Test Case: Valid Bulk Payload**
    valid_bulk_payload = (
        '{ "index" : { "_index" : "pytest1", "_id" : "1" } }\n'
        '{ "field1" : "value1" }\n'
        '{ "index" : { "_index" : "pytest1", "_id" : "2" } }\n'
        '{ "field1" : "value2" }'
    )

    resp_valid_bulk = session.post(url, data=valid_bulk_payload, headers=headers)
    print("Positive Test Response:", resp_valid_bulk.content)

    # Assert positive case
    assert resp_valid_bulk.status_code == 200, (
        f"Expected status code 200 for valid bulk request, but got {resp_valid_bulk.status_code}. "
        f"Response: {resp_valid_bulk.content}"
    )
    assert not resp_valid_bulk.json().get("errors"), (
        f"Expected no errors in the valid bulk request response, but got {resp_valid_bulk.json()}"
    )

    # **Negative Test Case 2: Invalid Bulk Payload (Empty Index)**
    invalid_bulk_payload2 = [
        { "index" : { "_index" : "" } },  # Empty index
        { "field1" : "value1" }
    ]

    # Convert the payload to JSON
    invalid_bulk_payload2_json = '\n'.join([json.dumps(item) for item in invalid_bulk_payload2])

    # Posting the payload to the API
    resp_invalid_bulk_2 = session.post(url, data=invalid_bulk_payload2_json, headers=headers)

    # Print the raw response and content for debugging
    print("Negative Test Response 2 (Raw Content):", resp_invalid_bulk_2.content)

    # Try to parse and print the JSON response
    try:
        response_json = resp_invalid_bulk_2.json()  # Attempt to parse the JSON response
        print("Negative Test Response 2 (JSON):", response_json)
    except Exception as e:
        print("Error parsing JSON response:", str(e))

    # Assert that we get a 200 status code (as observed in Postman behavior)
    assert resp_invalid_bulk_2.status_code == 200, (
        f"Expected status code 200 for invalid bulk request, but got {resp_invalid_bulk_2.status_code}. "
        f"Response: {resp_invalid_bulk_2.content}"
    )

    # Assert errors is true in the response
    assert resp_invalid_bulk_2.json().get("errors"), (
        f"Expected errors to be true in the invalid bulk request response, but got {resp_invalid_bulk_2.json()}"
    )


     # **Negative Test Case: Invalid index name**
    invalid_bulk_payload3 = (
        '{ "index" : { "_index" : "****", "_id" : "1" } }\n'
        '{ "field1" : "value1" }\n'
        '{ "index" : { "_index" : "****", "_id" : "2" } }\n'
        '{ "field1" : "value2" }'
    )

    resp_valid_bulk = session.post(url, data=valid_bulk_payload, headers=headers)
    print("Negative Test Response:", resp_valid_bulk.content)

    # Assert positive case
    assert resp_valid_bulk.status_code == 200, (
        f"Expected status code 200 for valid bulk request, but got {resp_valid_bulk.status_code}. "
        f"Response: {resp_valid_bulk.content}"
    )
  
