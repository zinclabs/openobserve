
def test_e2e_alerts(create_session, base_url):
    """Running an E2E test for get all the alerts list."""

    session = create_session
    url = base_url
    org_id = "default"

    resp_get_allalerts = session.get(f"{url}api/{org_id}/alerts")

    print(resp_get_allalerts.content)
    assert (
        resp_get_allalerts.status_code == 200
    ), f"Get all alerts list 200, but got {resp_get_allalerts.status_code} {resp_get_allalerts.content}"


def test_e2e_destinations(create_session, base_url):
    """Running an E2E test for get all the destination list under alerts."""

    session = create_session
    url = base_url
    org_id = "default"

    resp_get_alldestinations = session.get(f"{url}api/{org_id}/alerts/destinations")

    print(resp_get_alldestinations.content)
    assert (
        resp_get_alldestinations.status_code == 200
    ), f"Get all alerts list 200, but got {resp_get_alldestinations.status_code} {resp_get_alldestinations.content}"


def test_e2e_deleteinvaliddestination(create_session, base_url):
    """Running an E2E test for deleting destination that does not exist ."""

    session = create_session
    url = base_url
    org_id = "default"
    resp_delete_destination = session.delete(
        f"{url}api/{org_id}/alerts/destinations/destinationname"
    )
    assert (
        resp_delete_destination.status_code == 404
    ), f"Deleting this destination, but got {resp_delete_destination.status_code} {resp_delete_destination.content}"


def test_e2e_templates(create_session, base_url):
    """Running an E2E test for get all the alerts list."""

    session = create_session
    url = base_url
    org_id = "default"

    resp_get_alltemplates = session.get(f"{url}api/{org_id}/alerts/templates")

    print(resp_get_alltemplates.content)
    assert (
        resp_get_alltemplates.status_code == 200
    ), f"Get all alerts list 200, but got {resp_get_alltemplates.status_code} {resp_get_alltemplates.content}"


# TODO - Change body and add 2 other testcases once bug #1702 is fixed
def test_e2e_templatescreation(create_session, base_url):
    """Running an E2E test for get all the alerts list."""
    template_name = "newtemp"

    session = create_session
    url = base_url
    org_id = "default"
    payload = {"body": "invalid", "ise2e": True, "name": template_name}
    # create template under alerts
    resp_get_alltemplates = session.post(
        f"{url}api/{org_id}/alerts/templates", json=payload
    )

    print(resp_get_alltemplates.content)
    assert (
        resp_get_alltemplates.status_code == 200
    ), f"Createtemplate 200, but got {resp_get_alltemplates.status_code} {resp_get_alltemplates.content}"
    # delete created template
    resp_delete_function = session.delete(
        f"{base_url}api/{org_id}/alerts/templates/{template_name}"
    )
    assert (
        resp_delete_function.status_code == 200
    ), f"Deleting this template, but got {resp_delete_function.status_code} {resp_delete_function.content}"


# TODO - Add this testcase once bug #1703 is fixed
# def test_e2e_templatesinvaliddelete(create_session,base_url):
#     """Running an E2E test for get all the alerts list."""
#     template_name = "newtemp"

#     session = create_session
#     url = base_url
#     org_id = "e2e"
#     resp_delete_function = session.delete(f"{base_url}api/{org_id}/alerts/templates/{template_name}")
#     assert resp_delete_function.status_code == 200, f"Deleting this template, but got {resp_delete_function.status_code} {resp_delete_function.content}"


def test_e2e_createdestination(create_session, base_url):
    """Running an E2E test for create a new destination."""

    session = create_session
    url = base_url
    org_id = "default"
    skip_tls_verify_value = False

    headers = {"Content-Type": "application/json", "Custom-Header": "value"}

    payload = {
        "name": "pytesttemplate",
        "body": """
            {{
                "text": "{alert_name} is active"
            }}
        """.format(
            alert_name="pytestautomate"
        ),
    }

    # createtemplate
    resp_create_destinations = session.post(
        f"{url}api/{org_id}/alerts/templates",
        json=payload,
        headers=headers,
    )
    print(resp_create_destinations.content)
    
    destination_name = "py-destinations"
    payload = {
        "url": "www",
        "method": "post",
        "skip_tls_verify": skip_tls_verify_value,
        "template": "pytesttemplate",
        "headers": {"test": "test"},
        "name":"py-destinations"
    }
   
    # create destination
    resp_create_destinations = session.post(
        f"{url}api/{org_id}/alerts/destinations",
        json=payload,
        headers=headers,
    )
    print(resp_create_destinations.content)
    # get destination
    resp_create_destinations = session.get(
        f"{url}api/{org_id}/alerts/destinations/{destination_name}",
        json=payload,
        headers=headers,
    )
    print(resp_create_destinations.content)
    assert (
        resp_create_destinations.status_code == 200
    ), f"Get all alerts list 200, but got {resp_create_destinations.status_code} {resp_create_destinations.content}"

    # ingest logs
    session = create_session
    url = base_url
    org_id = "default"
    stream_name = "newpy_tests"
    payload = [
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
        f"{url}api/{org_id}/{stream_name}/_json", json=payload
    )

    print(resp_create_logstream.content)
    assert (
        resp_create_logstream.status_code == 200
    ), f"Get all alerts list 200, but got {resp_create_logstream.status_code} {resp_create_logstream.content}"

    stream_name = "newpy_tests"
    alert_name = "py-alert"
    is_real_time = False
    payload = {
        "name": alert_name,
        "stream_type": "logs",
        "stream_name": "newpy_tests",
        "is_real_time": is_real_time,
        "query_condition": {
            "conditions": [
                {
                    "column": "city",
                    "operator": "=",
                    "value": "200",
                    "id": "ebab5c0f-e78b-46b4-900a-22eb8a1f662c",
                }
            ],
            "sql": "",
            "promql": None,
            "type": "custom",
            "aggregation": None,
        },
        "trigger_condition": {
            "period": 10,
            "operator": ">=",
            "threshold": 3,
            "silence": 10,
        },
        "destinations": ["py-destinations"],
        "context_attributes": {},
        "enabled": True,
        "description": "",
    }
    resp_create_alert = session.post(
        f"{url}api/{org_id}/{stream_name}/alerts",
        json=payload,
        headers=headers,
    )
    print(resp_create_alert.content)
    assert (
        resp_create_alert.status_code == 200
    ), f"Get all alerts list 200, but got {resp_create_alert.status_code} {resp_create_alert.content}"
    resp_delete_alert = session.delete(
        f"{url}api/{org_id}/{stream_name}/alerts/{alert_name}?type=logs"
    )
    assert (
        resp_delete_alert.status_code == 200
    ), f"Deleting this function, but got {resp_delete_alert.status_code} {resp_delete_alert.content}"

    resp_delete_destination = session.delete(
        f"{url}api/{org_id}/alerts/destinations/{destination_name}"
    )
    assert (
        resp_delete_destination.status_code == 200
    ), f"Deleting this function, but got {resp_delete_destination.status_code} {resp_delete_destination.content}"

    resp_delete_template = session.delete(
        f"{url}api/{org_id}/alerts/templates/pytesttemplate"
    )
    assert (
        resp_delete_template.status_code == 200
    ), f"Deleting this function, but got {resp_delete_template.status_code} {resp_delete_template.content}"


def test_e2e_createalertsql(create_session, base_url):
    """Running an E2E test for create a new destination."""

    session = create_session
    url = base_url
    org_id = "default"
    skip_tls_verify_value = False

    headers = {"Content-Type": "application/json", "Custom-Header": "value"}

    payload = {
        "name": "pytesttemplate",
        "body": """
            {{
                "text": "{alert_name} is active"
            }}
        """.format(
            alert_name="pytestautomate"
        ),
    }

    # createtemplate
    resp_create_destinations = session.post(
        f"{url}api/{org_id}/alerts/templates",
        json=payload,
        headers=headers,
    )
    print(resp_create_destinations.content)
    
    destination_name = "py-destinations"
    payload = {
        "url": "www",
        "method": "post",
        "skip_tls_verify": skip_tls_verify_value,
        "template": "pytesttemplate",
        "headers": {"test": "test"},
        "name":"py-destinations"
    }
   
    # create destination
    resp_create_destinations = session.post(
        f"{url}api/{org_id}/alerts/destinations",
        json=payload,
        headers=headers,
    )
    print(resp_create_destinations.content)
    # get destination
    resp_create_destinations = session.get(
        f"{url}api/{org_id}/alerts/destinations/{destination_name}",
        json=payload,
        headers=headers,
    )
    print(resp_create_destinations.content)
    assert (
        resp_create_destinations.status_code == 200
    ), f"Get all alerts list 200, but got {resp_create_destinations.status_code} {resp_create_destinations.content}"

    # ingest logs
    session = create_session
    url = base_url
    org_id = "default"
    stream_name = "newpy_tests"
    payload = [
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
        f"{url}api/{org_id}/{stream_name}/_json", json=payload
    )

    print(resp_create_logstream.content)
    assert (
        resp_create_logstream.status_code == 200
    ), f"Get all alerts list 200, but got {resp_create_logstream.status_code} {resp_create_logstream.content}"

    stream_name = "newpy_tests"
    alert_name = "py-alert"
    is_real_time = False
    payload = {
        "name": alert_name,
        "stream_type": "logs",
        "stream_name": "newpy_tests",
        "is_real_time": is_real_time,
       "query_condition": {
            "type": "sql",
            "conditions": [],
            "sql": "SELECT kubernetes_container_name,code FROM \"stream_pytest_data\" ",
            "promql": "",
            "promql_condition":None,
            "aggregation": None,
            "vrl_function": None,
            "search_event_type": None,
            "multi_time_range": None
        },
        "trigger_condition": {
            "period": 10,
            "operator": ">=",
            "threshold": 3,
            "silence": 10,
        },
        "destinations": ["py-destinations"],
        "context_attributes": {},
        "enabled": True,
        "description": "",
    }
    resp_create_alert = session.post(
        f"{url}api/{org_id}/{stream_name}/alerts",
        json=payload,
        headers=headers,
    )
    print(resp_create_alert.content)
    assert (
        resp_create_alert.status_code == 200
    ), f"Get all alerts list 200, but got {resp_create_alert.status_code} {resp_create_alert.content}"
    resp_delete_alert = session.delete(
        f"{url}api/{org_id}/{stream_name}/alerts/{alert_name}?type=logs"
    )
    assert (
        resp_delete_alert.status_code == 200
    ), f"Deleting this function, but got {resp_delete_alert.status_code} {resp_delete_alert.content}"

    resp_delete_destination = session.delete(
        f"{url}api/{org_id}/alerts/destinations/{destination_name}"
    )
    assert (
        resp_delete_destination.status_code == 200
    ), f"Deleting this function, but got {resp_delete_destination.status_code} {resp_delete_destination.content}"

    resp_delete_template = session.delete(
        f"{url}api/{org_id}/alerts/templates/pytesttemplate"
    )
    assert (
        resp_delete_template.status_code == 200
    ), f"Deleting this function, but got {resp_delete_template.status_code} {resp_delete_template.content}"



def test_e2e_createalertfloat(create_session, base_url):
    """Running an E2E test for create a new destination."""

    session = create_session
    url = base_url
    org_id = "default"
    skip_tls_verify_value = False

    headers = {"Content-Type": "application/json", "Custom-Header": "value"}

    payload = {
        "name": "pytesttemplate",
        "body": """
            {{
                "text": "{alert_name} is active"
            }}
        """.format(
            alert_name="pytestautomate"
        ),
    }

    # createtemplate
    resp_create_destinations = session.post(
        f"{url}api/{org_id}/alerts/templates",
        json=payload,
        headers=headers,
    )
    print(resp_create_destinations.content)
    
    destination_name = "py-destinations"
    payload = {
        "url": "www",
        "method": "post",
        "skip_tls_verify": skip_tls_verify_value,
        "template": "pytesttemplate",
        "headers": {"test": "test"},
        "name":"py-destinations"
    }
   
    # create destination
    resp_create_destinations = session.post(
        f"{url}api/{org_id}/alerts/destinations",
        json=payload,
        headers=headers,
    )
    print(resp_create_destinations.content)
    # get destination
    resp_create_destinations = session.get(
        f"{url}api/{org_id}/alerts/destinations/{destination_name}",
        json=payload,
        headers=headers,
    )
    print(resp_create_destinations.content)
    assert (
        resp_create_destinations.status_code == 200
    ), f"Get all alerts list 200, but got {resp_create_destinations.status_code} {resp_create_destinations.content}"

    # ingest logs
    session = create_session
    url = base_url
    org_id = "default"
    stream_name = "newpy_tests"
    payload = [
          {
        "Athlete": "Michael Phelps",
        "City": "Beijing",
        "Country": "USA",
        "Discipline": "Swimming",
        "Sport": "Aquatics",
        "Year": 2008,
        "FloatValue": 23.45
    },
    {
        "Athlete": "Katie Ledecky",
        "City": "Rio de Janeiro",
        "Country": "USA",
        "Discipline": "Swimming",
        "Sport": "Aquatics",
        "Year": 2016,
        "FloatValue": 16.8
    },
    ]

    resp_create_logstream = session.post(
        f"{url}api/{org_id}/{stream_name}/_json", json=payload
    )

    print(resp_create_logstream.content)
    assert (
        resp_create_logstream.status_code == 200
    ), f"Get all alerts list 200, but got {resp_create_logstream.status_code} {resp_create_logstream.content}"

    stream_name = "newpy_tests"
    alert_name = "py-alert"
    is_real_time = False
    payload = {
        "name": alert_name,
        "stream_type": "logs",
        "stream_name": "newpy_tests",
        "is_real_time": is_real_time,
       "query_condition": {
            "type": "sql",
            "conditions": [],
            "sql": "SELECT athlete,city FROM \"newpy_tests\" where floatvalue = 16.8",
            "promql": "",
            "promql_condition":None,
            "aggregation": None,
            "vrl_function": None,
            "search_event_type": None,
            "multi_time_range": None
        },
        "trigger_condition": {
            "period": 10,
            "operator": ">=",
            "threshold": 3,
            "silence": 10,
        },
        "destinations": ["py-destinations"],
        "context_attributes": {},
        "enabled": True,
        "description": "",
    }
    resp_create_alert = session.post(
        f"{url}api/{org_id}/{stream_name}/alerts",
        json=payload,
        headers=headers,
    )
    print(resp_create_alert.content)
    assert (
        resp_create_alert.status_code == 200
    ), f"Get all alerts list 200, but got {resp_create_alert.status_code} {resp_create_alert.content}"
    resp_delete_alert = session.delete(
        f"{url}api/{org_id}/{stream_name}/alerts/{alert_name}?type=logs"
    )
    assert (
        resp_delete_alert.status_code == 200
    ), f"Deleting this function, but got {resp_delete_alert.status_code} {resp_delete_alert.content}"

    resp_delete_destination = session.delete(
        f"{url}api/{org_id}/alerts/destinations/{destination_name}"
    )
    assert (
        resp_delete_destination.status_code == 200
    ), f"Deleting this function, but got {resp_delete_destination.status_code} {resp_delete_destination.content}"

    resp_delete_template = session.delete(
        f"{url}api/{org_id}/alerts/templates/pytesttemplate"
    )
    assert (
        resp_delete_template.status_code == 200
    ), f"Deleting this function, but got {resp_delete_template.status_code} {resp_delete_template.content}"









