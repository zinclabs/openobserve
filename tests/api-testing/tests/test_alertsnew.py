import time
import random
import uuid
import pytest
import json
import requests


def test_get_alertsnew(create_session, base_url):
    """Running an E2E test for getting all the new alerts list."""

    session = create_session
    url = base_url
    org_id = "default"

    resp_get_allalertsnew = session.get(f"{url}api/v2/{org_id}/alerts")

    print(resp_get_allalertsnew.content)
    assert (
        resp_get_allalertsnew.status_code == 200
    ), f"Get all new alerts list 200, but got {resp_get_allalertsnew.status_code} {resp_get_allalertsnew.content}"

def test_new_alert_create(create_session, base_url):
    """Running an E2E test for create a new alert."""

    alert_name = f"newalert_{random.randint(1000, 9999)}"  # Make the name unique
    folder_alert = f"newfloder_{random.randint(1000, 9999)}"  # Make the name unique
    
    session = create_session
    url = base_url
    org_id = "default"


    headers = {"Content-Type": "application/json", "Custom-Header": "value"}


    # create folder

    payload_folder = {"description": "newfoldernvp", "name": folder_alert}
    resp_create_folder = session.post(
        f"{url}api/v2/{org_id}/folders/alerts", json=payload_folder)

    print(resp_create_folder.content)

    folder_id = resp_create_folder.json()["folderId"]

    assert (
        resp_create_folder.status_code == 200
    ), f"Expected 200, but got {resp_create_folder.status_code} {resp_create_folder.content}"

    # createtemplate
    template_alert = f"newtemp_{random.randint(10000, 99999)}"  # Make the name unique
    destination_alert = f"newdest_{random.randint(10000, 99999)}"  # Make the name unique


    payload_temp_alert = {
        "name":template_alert,
        "body":"{\n  \"text\": \"For stream {stream_name} of organization {org_name} alert {alert_name} of type {alert_type} is active\"\n}",
        "type":"http",
        "title":""
        }
    
    # Create template for alerts
    resp_create_templates_alert = session.post(
        f"{url}api/{org_id}/alerts/templates", json=payload_temp_alert
    )

    print(resp_create_templates_alert.content)
    assert (
        resp_create_templates_alert.status_code == 200
    ), f"Create template 200, but got {resp_create_templates_alert.status_code} {resp_create_templates_alert.content}"

    
    skip_tls_verify_value = False  # Define the skip_tls_verify_value

    payload_dest_alert = {
        "url":"https://jsonplaceholder.typicode.com/todos",
        "method":"get",
        "skip_tls_verify":skip_tls_verify_value,
        "template":template_alert,
        "headers":{},
        "name":destination_alert
        }
    

    # Create destination
    resp_create_destinations_alert = session.post(
        f"{url}api/{org_id}/alerts/destinations",
        json=payload_dest_alert,
    )
    print(resp_create_destinations_alert.content)

    assert (
        resp_create_destinations_alert.status_code == 200
    ), f"Failed to create destination: {resp_create_destinations_alert.status_code} {resp_create_destinations_alert.content}"

    # Get destination
    resp_get_destinations_alert = session.get(
    f"{url}api/{org_id}/alerts/destinations/{destination_alert}"
    )
    print(resp_get_destinations_alert.content)

    assert (
    resp_get_destinations_alert.status_code == 200
    ), f"Get all destinations list 200, but got {resp_get_destinations_alert.status_code} {resp_get_destinations_alert.content}"

    # Check if the destination name matches
    assert (
    resp_get_destinations_alert.json()["name"] == destination_alert
    ), f"Destination {destination_alert} not found in the response"

    time.sleep(5)

    # ingest logs
    stream_name = "default"
    payload_logs = [
        {
            "Athlete": "newtemp",
            "City": "Athens",
            "Country": "HUN",
            "Discipline": "Swimming",
            "Sport": "Aquatics",
            "Year": 1896,
        },
        {
            "Athlete": "HERSCHMANN",
            "City": "Athens",
            "Country": "CHN",
            "Discipline": "Swimming",
            "Sport": "Aquatics",
            "Year": 1896,
        },
    ]

    resp_create_logstream = session.post(
        f"{url}api/{org_id}/{stream_name}/_json", json=payload_logs
    )

    print(resp_create_logstream.content)
    assert (
        resp_create_logstream.status_code == 200
    ), f"Get all logs list 200, but got {resp_create_logstream.status_code} {resp_create_logstream.content}"
    
    time.sleep(5)  
    payload_alert = {
    "name": alert_name,
    "row_template": template_alert,
    "stream_type": "logs",
    "stream_name": "default",
    "is_real_time": False,
    "context_attributes": {},
    "query_condition": {
        "conditions": [ {
        "column": "log",
        "operator": "=",
        "value": "200",
        "type": None,  # Set it to None or remove it
        "id": "95eb852b-b93b-4cb6-ab17-b7608b4fc741"
        } ],
        "search_event_type": "ui",
        "sql": "",
        "promql": "",
        "type": "custom",
        "promql_condition": None,
        "vrl_function": None,
        "multi_time_range": []
    },
    "trigger_condition": {
        "period": 10,
        "operator": ">=",
        "frequency": 1,
        "cron": "",
        "threshold": 3,
        "silence": 10,
        "frequency_type": "minutes",
        "timezone": "UTC",
        "tolerance_in_secs": 0
    },
    "org_id": "default",
    "destinations": [destination_alert],
    "context_attributes": {},
    "enabled": True,
    "description": "test",
    "owner": "root@example.com",
    "folderId": folder_id
 }
    resp_post_alertnew = session.post(
    f"{url}api/v2/{org_id}/alerts",
    json=payload_alert,
    headers=headers,
    )
    print(resp_post_alertnew.content)

    assert ( resp_post_alertnew.status_code == 200  
    ), f"Post alert expected 200, but got {resp_post_alertnew.status_code} {resp_post_alertnew.content}"
    # assert (resp_post_alertnew.json()['id'] == alert_id), f"Alert id
    # mismatch {resp_post_alertnew.json()['id']} {alert_id}"
    print(f"Alert {alert_name} created successfully")


def test_get_alertnew(create_session, base_url):
    """Running an E2E test for getting particular new alert ID."""

    session = create_session
    url = base_url
    org_id = "default"

    # Make the request to get the list of alerts
    resp_get_allalertsnew = session.get(f"{url}api/v2/{org_id}/alerts")

    # Ensure the response is successful
    assert resp_get_allalertsnew.status_code == 200, f"Failed to fetch alerts: {resp_get_allalertsnew.status_code}"

    # Parse the response JSON
    response_json = resp_get_allalertsnew.json()

    # Check if "list" is in the response and proceed
    assert "list" in response_json, "Response does not contain 'list'"

    # Get the list of alerts from the response
    alerts = response_json["list"]

    # Now you can iterate over the alerts
    for alert in alerts:
        alert_id = alert.get("alert_id")
        assert alert_id, f"Alert ID is missing for alert: {alert}"

        print(f"Extracted alert_id: {alert_id}")

    # Get  request using the extracted alert_id
        resp_get_alertnew = session.get(f"{url}api/v2/{org_id}/alerts/{alert_id}")
        assert resp_get_alertnew.status_code == 200, f"Failed to get details for alert {alert_id}"
        print(f"Successfully fetched details for alert {alert_id}")

def test_put_alertnew_update(create_session, base_url):
    """Running an E2E test for updating the new alert."""

    alert_name = f"newalert_{random.randint(1000, 9999)}"  # Make the name unique
    folder_alert = f"newfloder_{random.randint(1000, 9999)}"  # Make the name unique
    
    session = create_session
    url = base_url
    org_id = "default"


    headers = {"Content-Type": "application/json", "Custom-Header": "value"}


    # create folder

    payload_folder = {"description": "newfoldernvp", "name": folder_alert}
    resp_create_folder = session.post(
        f"{url}api/v2/{org_id}/folders/alerts", json=payload_folder)

    print(resp_create_folder.content)

    folder_id = resp_create_folder.json()["folderId"]

    assert (
        resp_create_folder.status_code == 200
    ), f"Expected 200, but got {resp_create_folder.status_code} {resp_create_folder.content}"

    # createtemplate
    template_alert = f"newtemp_{random.randint(10000, 99999)}"  # Make the name unique
    destination_alert = f"newdest_{random.randint(10000, 99999)}"  # Make the name unique


    payload_temp_alert = {
        "name":template_alert,
        "body":"{\n  \"text\": \"For stream {stream_name} of organization {org_name} alert {alert_name} of type {alert_type} is active\"\n}",
        "type":"http",
        "title":""
        }
    
    # Create template for alerts
    resp_create_templates_alert = session.post(
        f"{url}api/{org_id}/alerts/templates", json=payload_temp_alert
    )

    print(resp_create_templates_alert.content)
    assert (
        resp_create_templates_alert.status_code == 200
    ), f"Create template 200, but got {resp_create_templates_alert.status_code} {resp_create_templates_alert.content}"

    
    skip_tls_verify_value = False  # Define the skip_tls_verify_value

    payload_dest_alert = {
        "url":"https://jsonplaceholder.typicode.com/todos",
        "method":"get",
        "skip_tls_verify":skip_tls_verify_value,
        "template":template_alert,
        "headers":{},
        "name":destination_alert
        }
    

    # Create destination
    resp_create_destinations_alert = session.post(
        f"{url}api/{org_id}/alerts/destinations",
        json=payload_dest_alert,
    )
    print(resp_create_destinations_alert.content)

    assert (
        resp_create_destinations_alert.status_code == 200
    ), f"Failed to create destination: {resp_create_destinations_alert.status_code} {resp_create_destinations_alert.content}"

    # Get destination
    resp_get_destinations_alert = session.get(
    f"{url}api/{org_id}/alerts/destinations/{destination_alert}"
    )
    print(resp_get_destinations_alert.content)

    assert (
    resp_get_destinations_alert.status_code == 200
    ), f"Get all destinations list 200, but got {resp_get_destinations_alert.status_code} {resp_get_destinations_alert.content}"

    # Check if the destination name matches
    assert (
    resp_get_destinations_alert.json()["name"] == destination_alert
    ), f"Destination {destination_alert} not found in the response"

    time.sleep(5)

    # ingest logs
    stream_name = "default"
    payload_logs = [
        {
            "Athlete": "newtemp",
            "City": "Athens",
            "Country": "HUN",
            "Discipline": "Swimming",
            "Sport": "Aquatics",
            "Year": 1896,
        },
        {
            "Athlete": "HERSCHMANN",
            "City": "Athens",
            "Country": "CHN",
            "Discipline": "Swimming",
            "Sport": "Aquatics",
            "Year": 1896,
        },
    ]

    resp_create_logstream = session.post(
        f"{url}api/{org_id}/{stream_name}/_json", json=payload_logs
    )

    print(resp_create_logstream.content)
    assert (
        resp_create_logstream.status_code == 200
    ), f"Get all logs list 200, but got {resp_create_logstream.status_code} {resp_create_logstream.content}"
    
    time.sleep(5)  
    payload_alert = {
    "name": alert_name,
    "row_template": template_alert,
    "stream_type": "logs",
    "stream_name": "default",
    "is_real_time": False,
    "context_attributes": {},
    "query_condition": {
        "conditions": [ {
        "column": "log",
        "operator": "=",
        "value": "200",
        "type": None,  # Set it to None or remove it
        "id": "95eb852b-b93b-4cb6-ab17-b7608b4fc741"
        } ],
        "search_event_type": "ui",
        "sql": "",
        "promql": "",
        "type": "custom",
        "promql_condition": None,
        "vrl_function": None,
        "multi_time_range": []
    },
    "trigger_condition": {
        "period": 10,
        "operator": ">=",
        "frequency": 1,
        "cron": "",
        "threshold": 3,
        "silence": 10,
        "frequency_type": "minutes",
        "timezone": "UTC",
        "tolerance_in_secs": 0
    },
    "org_id": "default",
    "destinations": [destination_alert],
    "context_attributes": {},
    "enabled": True,
    "description": "test",
    "owner": "root@example.com",
    "folderId": folder_id
    }
    resp_post_alertnew = session.post(
    f"{url}api/v2/{org_id}/alerts",
    json=payload_alert,
    headers=headers,
    )
    print(resp_post_alertnew.content)
    assert resp_post_alertnew.status_code == 200


    # Make the request to get the list of alerts
    resp_get_allalertsnew = session.get(f"{url}api/v2/{org_id}/alerts")

    # Ensure the response is successful
    assert resp_get_allalertsnew.status_code == 200, f"Failed to fetch alerts: {resp_get_allalertsnew.status_code}"

    # Parse the response JSON
    response_json = resp_get_allalertsnew.json()

    # Check if "list" is in the response and proceed
    assert "list" in response_json, "Response does not contain 'list'"

    # Get the list of alerts from the response
    alerts = response_json["list"]

    # Now you can iterate over the alerts
    for alert in alerts:
        alert_id = alert.get("alert_id")
        assert alert_id, f"Alert ID is missing for alert: {alert}"

        print(f"Extracted alert_id: {alert_id}")

    # Validate the alert existence first
        resp_check_alert = session.get(f"{url}api/v2/{org_id}/alerts/{alert_id}")
        assert resp_check_alert.status_code == 200, f"Alert {alert_id} does not exist or cannot be retrieved."
        print(f"Alert {alert_id} exists and is retrievable.")

    # Update the alert
        payload_alert_update = {
        "id": alert_id,
        "name": alert_name,
        "org_id": "default",
        "stream_type": "logs",
        "stream_name": "default",
        "is_real_time": False,
        "query_condition": {
        "type": "custom",
        "conditions": [
        {
        "column": "job",
        "operator": "=",
        "value": "test",
        "ignore_case": False,
        "id": "aba20f7f-8006-40c5-9123-e880ecda1c95"
        }
        ],
        "sql": None,
        "promql": None,
        "promql_condition": None,
        "aggregation": None,
        "vrl_function": None,
        "search_event_type": None,
        "multi_time_range": []
        },
        "trigger_condition": {
        "period": 10,
        "operator": ">=",
        "threshold": 3,
        "frequency": 1,
        "cron": "",
        "frequency_type": "minutes",
        "silence": 10,
        "timezone": "UTC",
        "tolerance_in_secs": None
        },
        "destinations": [destination_alert],
        "context_attributes": {},
        "row_template": "",
        "description": "Test Updated",
        "enabled": True,
        "tz_offset": 0,
        "last_triggered_at": None,
        "last_satisfied_at": None,
        "owner": "root@example.com",
        "updated_at": 1736486401000,
        "last_edited_by": "root@example.com",
        "createdAt": 1736486401000,
        "lastTriggeredAt": 1736767838564,
        "lastEditedBy": "root@example.com"
        } 
        resp_put_alertnew = session.put(
        f"{url}api/v2/{org_id}/alerts/{alert_id}?type=logs",
        json=payload_alert_update,
        headers=headers,
         )
        print(resp_put_alertnew.content)

        assert ( resp_put_alertnew.status_code == 200  
        ), f"Post alert expected 200, but got {resp_put_alertnew.status_code} {resp_put_alertnew.content}"

def test_put_alertnew_disable(create_session, base_url):
    """Running an E2E test for getting the new alert disable."""

    session = create_session
    url = base_url
    org_id = "default"

    # Make the request to get the list of alerts
    resp_get_allalertsnew = session.get(f"{url}api/v2/{org_id}/alerts")

    # Ensure the response is successful
    assert resp_get_allalertsnew.status_code == 200, f"Failed to fetch alerts: {resp_get_allalertsnew.status_code}"

    # Parse the response JSON
    response_json = resp_get_allalertsnew.json()

    # Check if "list" is in the response and proceed
    assert "list" in response_json, "Response does not contain 'list'"

    # Get the list of alerts from the response
    alerts = response_json["list"]

    # Now you can iterate over the alerts
    for alert in alerts:
        alert_id = alert.get("alert_id")
        assert alert_id, f"Alert ID is missing for alert: {alert}"

        print(f"Extracted alert_id: {alert_id}")

    # Validate the alert existence first
        resp_check_alert = session.get(f"{url}api/v2/{org_id}/alerts/{alert_id}")
        assert resp_check_alert.status_code == 200, f"Alert {alert_id} does not exist or cannot be retrieved."
        print(f"Alert {alert_id} exists and is retrievable.")

    
    # Proceed to disable the alert
        resp_alertnew_disable = session.put(f"{url}api/v2/{org_id}/alerts/{alert_id}/enable?value=false&type=logs")
        print(f"Disable Alert Response: {resp_alertnew_disable.text}")
        assert resp_alertnew_disable.status_code == 200, f"Failed to disable alert {alert_id}"
        print(f"Successfully disabled alert {alert_id}")

def test_put_alertnew_enable(create_session, base_url):
    """Running an E2E test for getting the new alert enable."""

    session = create_session
    url = base_url
    org_id = "default"

    # Make the request to get the list of alerts
    resp_get_allalertsnew = session.get(f"{url}api/v2/{org_id}/alerts")

    # Ensure the response is successful
    assert resp_get_allalertsnew.status_code == 200, f"Failed to fetch alerts: {resp_get_allalertsnew.status_code}"

    # Parse the response JSON
    response_json = resp_get_allalertsnew.json()

    # Check if "list" is in the response and proceed
    assert "list" in response_json, "Response does not contain 'list'"

    # Get the list of alerts from the response
    alerts = response_json["list"]

    # Now you can iterate over the alerts
    for alert in alerts:
        alert_id = alert.get("alert_id")
        assert alert_id, f"Alert ID is missing for alert: {alert}"

        print(f"Extracted alert_id: {alert_id}")

    # Validate the alert existence first
        resp_check_alert = session.get(f"{url}api/v2/{org_id}/alerts/{alert_id}")
        assert resp_check_alert.status_code == 200, f"Alert {alert_id} does not exist or cannot be retrieved."
        print(f"Alert {alert_id} exists and is retrievable.")


    # Proceed to enable the alert
        resp_alertnew_enable = session.put(f"{url}api/v2/{org_id}/alerts/{alert_id}/enable?value=true&type=logs")
        print(f"Enable Alert Response: {resp_alertnew_enable.text}")
        assert resp_alertnew_enable.status_code == 200, f"Failed to enable alert {alert_id}"
        print(f"Successfully enabled alert {alert_id}")

def test_put_alertnew_trigger(create_session, base_url):
    """Running an E2E test for getting the new alert trigger."""

    session = create_session
    url = base_url
    org_id = "default"

    # Make the request to get the list of alerts
    resp_get_allalertsnew = session.get(f"{url}api/v2/{org_id}/alerts")

    # Ensure the response is successful
    assert resp_get_allalertsnew.status_code == 200, f"Failed to fetch alerts: {resp_get_allalertsnew.status_code}"

    # Parse the response JSON
    response_json = resp_get_allalertsnew.json()

    # Check if "list" is in the response and proceed
    assert "list" in response_json, "Response does not contain 'list'"

    # Get the list of alerts from the response
    alerts = response_json["list"]

    # Now you can iterate over the alerts
    for alert in alerts:
        alert_id = alert.get("alert_id")
        assert alert_id, f"Alert ID is missing for alert: {alert}"

        print(f"Extracted alert_id: {alert_id}")

    # Validate the alert existence first
        resp_check_alert = session.get(f"{url}api/v2/{org_id}/alerts/{alert_id}")
        assert resp_check_alert.status_code == 200, f"Alert {alert_id} does not exist or cannot be retrieved."
        print(f"Alert {alert_id} exists and is retrievable.")

        time.sleep(5)  # Brief wait for status update

    # Trigger the alert
        print(f"Attempting to trigger alert with ID: {alert_id}")
        resp_alertnew_trigger = session.put(f"{url}api/v2/{org_id}/alerts/{alert_id}/trigger?type=logs")

    # Log response details for debugging
        print(f"Trigger Alert Response Status Code: {resp_alertnew_trigger.status_code}")
        print(f"Trigger Alert Response Body: {resp_alertnew_trigger.text}")

        print(f"Alert Trigger URL: {url}api/v2/{org_id}/alerts/{alert_id}/trigger?type=logs")
        print(f"Alert Details: {alert}")


    # Check if the response was successful
        assert resp_alertnew_trigger.status_code == 200, f"Failed to trigger alert {alert_id}"
        print(f"Successfully triggered alert {alert_id}")
    
def test_delete_alertnew(create_session, base_url):
    """Running an E2E test for deleting the new alert."""

    session = create_session
    url = base_url
    org_id = "default"

    # Make the request to get the list of alerts
    resp_get_allalertsnew = session.get(f"{url}api/v2/{org_id}/alerts")

    # Ensure the response is successful
    assert resp_get_allalertsnew.status_code == 200, f"Failed to fetch alerts: {resp_get_allalertsnew.status_code}"

    # Parse the response JSON
    response_json = resp_get_allalertsnew.json()

    # Check if "list" is in the response and proceed
    assert "list" in response_json, "Response does not contain 'list'"

    # Get the list of alerts from the response
    alerts = response_json["list"]

    # Now you can iterate over the alerts
    for alert in alerts:
        alert_id = alert.get("alert_id")
        assert alert_id, f"Alert ID is missing for alert: {alert}"

        print(f"Extracted alert_id: {alert_id}")

    # Validate the alert existence first
        resp_check_alert = session.get(f"{url}api/v2/{org_id}/alerts/{alert_id}")
        assert resp_check_alert.status_code == 200, f"Alert {alert_id} does not exist or cannot be retrieved."
        print(f"Alert {alert_id} exists and is retrievable.")

    # Proceed to delete the alert
        resp_delete_alertnew = session.delete(f"{url}api/v2/{org_id}/alerts/{alert_id}")
        print(f"Deleted Alert Response: {resp_delete_alertnew.text}")
        assert resp_delete_alertnew.status_code == 200, f"Failed to delete alert {alert_id}"
        print(f"Successfully deleted alert {alert_id}")
