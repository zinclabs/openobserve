import pytest

@pytest.mark.parametrize(
    "pipeline_name, stream_name, action, expected_status",
    [
        ("pipeline12222", "stream_pytest_data", None, 200),  # No action (default behavior)
        ("pipeline12345", "stream_test_data", "enable", 200),  # Action: Enable
        ("pipeline67890", "stream_other_data", "disable", 200),  # Action: Disable
    ]
)
def test_pipeline_creation_and_action_realtime(create_session, base_url, pipeline_name, stream_name, action, expected_status):
    """Running an E2E test for pipeline creation and enabling/disabling the pipeline."""
    session = create_session
    url = base_url
    org_id = "default"

    # Payload for pipeline creation
    pipeline_payload = {
        "name": pipeline_name,
        "description": "",
        "source": {"source_type": "realtime"},
        "nodes": [
            {
                "id": "1e1fa129-cfbd-4082-8d39-c4dc418a670f",
                "type": "input",
                "data": {
                    "node_type": "stream",
                    "stream_name": stream_name,
                    "stream_type": "logs",
                    "org_id": org_id,
                },
                "position": {"x": 169.83333, "y": 55},
                "io_type": "input",
            },
            {
                "id": "8e0ed123-7737-4801-9466-bd909c425c72",
                "type": "output",
                "data": {
                    "node_type": "stream",
                    "stream_name": stream_name,
                    "stream_type": "logs",
                    "org_id": org_id,
                },
                "position": {"x": 305.33334, "y": 276},
                "io_type": "output",
            },
        ],
        "edges": [
            {
                "id": "e1e1fa129-cfbd-4082-8d39-c4dc418a670f-8e0ed123-7737-4801-9466-bd909c425c72",
                "source": "1e1fa129-cfbd-4082-8d39-c4dc418a670f",
                "target": "8e0ed123-7737-4801-9466-bd909c425c72",
            }
        ],
        "org": org_id,
    }

    # Create a pipeline
    resp_create_pipeline = session.post(f"{url}api/{org_id}/pipelines", json=pipeline_payload)
    assert resp_create_pipeline.status_code == expected_status, f"Expected status code {expected_status} but got {resp_create_pipeline.status_code}"
    print("Create pipeline response:", resp_create_pipeline.json())

    # Retrieve the pipeline list
    resp_list_pipelines = session.get(f"{url}api/{org_id}/pipelines")
    assert resp_list_pipelines.status_code == 200, f"Expected status code 200 but got {resp_list_pipelines.status_code}"
    
    # Parse the pipeline list response
    pipelines_list = resp_list_pipelines.json().get("list", [])
    assert pipelines_list, "No pipelines found in the list response."

    # Extract the pipeline_id for the created pipeline
    pipeline_id = None
    for pipeline in pipelines_list:
        if pipeline["name"] == pipeline_name:
            pipeline_id = pipeline["pipeline_id"]
            break

    assert pipeline_id, "Pipeline ID not found for the created pipeline."
    print(f"Pipeline ID: {pipeline_id}")

    # Enable or disable the pipeline based on the action parameter
    if action == "enable":
        resp_enable_pipeline = session.put(f"{url}api/{org_id}/pipelines/{pipeline_id}/enable?value=true")
        assert resp_enable_pipeline.status_code == expected_status, f"Expected status code {expected_status} but got {resp_enable_pipeline.status_code}"
        print(f"Pipeline {pipeline_id} enabled successfully.")
    elif action == "disable":
        resp_enable_pipeline = session.put(f"{url}api/{org_id}/pipelines/{pipeline_id}/enable?value=false")
        assert resp_enable_pipeline.status_code == expected_status, f"Expected status code {expected_status} but got {resp_enable_pipeline.status_code}"
        print(f"Pipeline {pipeline_id} disabled successfully.")

     
    resp_get_pipeline = session.get(f"{url}api/{org_id}/pipelines")
    print(resp_get_pipeline.json())
    assert resp_get_pipeline.status_code == 200
    pipeline_list = resp_get_pipeline.json()["list"]
    target_pipeline = next((p for p in pipeline_list if p["pipeline_id"] == pipeline_id), None)
    assert target_pipeline is not None, f"Pipeline {pipeline_id} not found in the list"
    if action == "enable":
       assert target_pipeline["enabled"] is True
    if action == "disable":
       assert target_pipeline["enabled"] is False

    # Delete the pipeline
    resp_delete_pipeline = session.delete(f"{url}api/{org_id}/pipelines/{pipeline_id}")
    assert resp_delete_pipeline.status_code == 200, f"Expected status code 200 but got {resp_delete_pipeline.status_code}"
    # Verify pipeline is deleted
    resp_verify_deletion = session.get(f"{url}api/{org_id}/pipelines")
    assert resp_verify_deletion.status_code == 200
    deleted_pipeline = next((p for p in resp_verify_deletion.json()["list"] if p["pipeline_id"] == pipeline_id), None)
    assert deleted_pipeline is None, f"Pipeline {pipeline_id} still exists after deletion"


@pytest.mark.parametrize(
    "pipeline_name, stream_name, action, expected_status, schedule_enabled",
    [
        ("pipeline12222", "stream_pytest_data", None, 200, False),  # No action (default behavior)
        ("scheduled_pipeline", "stream_scheduled_data", "enable", 200, True),  # Action: Enable with schedule
        ("scheduled_pipeline", "stream_scheduled_data", "disable", 200, False),  # Action: Disable with schedule
    ]
)

def test_pipeline_creation_and_action_with_schedule(create_session, base_url, pipeline_name, stream_name, action, expected_status, schedule_enabled):
    """Running an E2E test for pipeline creation with scheduling and enabling/disabling the pipeline."""
    session = create_session
    url = base_url
    org_id = "default"

    # Payload for scheduled pipeline creation
    pipeline_payload = {
        "name": pipeline_name,
        "description": "",
        "source": {"source_type": "scheduled"},
        "nodes": [
            {
                "id": "8b03771c-2e94-4c2f-b902-8fb8b8e66df2",
                "type": "input",
                "dimensions": {"width": 140, "height": 42},
                "computedPosition": {"x": 212.1047849020086, "y": 93.00563781047852, "z": 0},
                "handleBounds": {
                    "source": [{"id": "output", "position": "bottom", "x": 55.03916432328342, "y": 36.00006521998462, "width": 30, "height": 10}],
                    "target": []
                },
                "position": {"x": 212.1047849020086, "y": 93.00563781047852},
                "data": {
                    "label": "8b03771c-2e94-4c2f-b902-8fb8b8e66df2",
                    "node_type": "query",
                    "stream_type": "logs",
                    "org_id": org_id,
                    "query_condition": {
                        "type": "sql",
                        "conditions": None,
                        "sql": "select * from default",
                        "promql": None,
                        "promql_condition": None,
                        "aggregation": None,
                        "vrl_function": None,
                        "search_event_type": "DerivedStream"
                    },
                    "trigger_condition": {
                        "period": 15,
                        "operator": "=",
                        "threshold": 0,
                        "frequency": 15,
                        "cron": "",
                        "frequency_type": "minutes",
                        "silence": 0
                    }
                },
                "io_type": "input"
            },
            {
                "id": "cfa476a0-24f3-4ca3-a14f-b56b04729922",
                "type": "output",
                "dimensions": {"width": 250, "height": 42},
                "computedPosition": {"x": 269.2781219770897, "y": 161.6932759313432, "z": 0},
                "handleBounds": {
                    "source": [],
                    "target": [{"id": "input", "position": "top", "x": 110.11741720560622, "y": -4.000002219844093, "width": 30, "height": 10}]
                },
                "position": {"x": 269.2781219770897, "y": 161.6932759313432},
                "data": {
                    "label": "cfa476a0-24f3-4ca3-a14f-b56b04729922",
                    "node_type": "stream",
                    "stream_type": "logs",
                    "stream_name": stream_name,
                    "org_id": org_id
                },
                "io_type": "output"
            }
        ],
        "edges": [
            {
                "id": "e8b03771c-2e94-4c2f-b902-8fb8b8e66df2-cfa476a0-24f3-4ca3-a14f-b56b04729922",
                "type": "custom",
                "source": "8b03771c-2e94-4c2f-b902-8fb8b8e66df2",
                "target": "cfa476a0-24f3-4ca3-a14f-b56b04729922",
                "data": {},
                "markerEnd": {"type": "arrowclosed", "width": 20, "height": 20},
                "style": {"strokeWidth": 2},
                "animated": True
            }
        ],
        "org": org_id,
        "schedule": {
            "enabled": schedule_enabled,
            "frequency": "daily",  # Example frequency, adjust as needed
            "time": "02:00",       # Example time, adjust as needed
        }
    }

    # Create a pipeline
    resp_create_pipeline = session.post(f"{url}api/{org_id}/pipelines", json=pipeline_payload)
    assert resp_create_pipeline.status_code == expected_status, f"Expected status code {expected_status} but got {resp_create_pipeline.status_code}"
    print("Create pipeline response:", resp_create_pipeline.json())

    # Retrieve the pipeline list
    resp_list_pipelines = session.get(f"{url}api/{org_id}/pipelines")
    assert resp_list_pipelines.status_code == 200, f"Expected status code 200 but got {resp_list_pipelines.status_code}"
    
    # Extract the pipeline_id for the created pipeline
    pipelines_list = resp_list_pipelines.json().get("list", [])
    pipeline_id = next((p["pipeline_id"] for p in pipelines_list if p["name"] == pipeline_name), None)
    assert pipeline_id, "Pipeline ID not found for the created pipeline."
    print(f"Pipeline ID: {pipeline_id}")

    # Enable/disable pipeline based on the action
    value = "true" if action == "enable" else "false"
    resp_toggle_pipeline = session.put(f"{url}api/{org_id}/pipelines/{pipeline_id}/enable?value={value}")
    assert resp_toggle_pipeline.status_code == expected_status, f"Expected status code {expected_status} but got {resp_toggle_pipeline.status_code}"
    print(f"Pipeline {pipeline_id} {'enabled' if action == 'enable' else 'disabled'} successfully.")

    # Verify the pipeline state
    resp_verify_pipeline = session.get(f"{url}api/{org_id}/pipelines")
    pipeline_state = next((p["enabled"] for p in resp_verify_pipeline.json()["list"] if p["pipeline_id"] == pipeline_id), None)
    assert pipeline_state == (action == "enable"), f"Pipeline state mismatch after {action} action."

    # Delete the pipeline
    resp_delete_pipeline = session.delete(f"{url}api/{org_id}/pipelines/{pipeline_id}")
    assert resp_delete_pipeline.status_code == 200, f"Expected status code 200 but got {resp_delete_pipeline.status_code}"
    print(f"Pipeline {pipeline_id} deleted successfully.")



@pytest.mark.parametrize(
    "pipeline_name, stream_name, action, expected_status, schedule_enabled",
    [
        ("pipeline12222", "stream_pytest_data", None, 400, False),  # No action (default behavior)
     
    ]
)

def test_pipeline_creation_and_action_with_schedule(create_session, base_url, pipeline_name, stream_name, action, expected_status, schedule_enabled):
    """Running an E2E test for pipeline creation with scheduling and enabling/disabling the pipeline."""
    session = create_session
    url = base_url
    org_id = "default"

    # Payload for scheduled pipeline creation
    pipeline_payload = {
        "name": pipeline_name,
        "description": "",
        "source": {"source_type": "scheduled"},
        "nodes": [
            {
                "id": "8b03771c-2e94-4c2f-b902-8fb8b8e66df2",
                "type": "input",
                "dimensions": {"width": 140, "height": 42},
                "computedPosition": {"x": 212.1047849020086, "y": 93.00563781047852, "z": 0},
                "handleBounds": {
                    "source": [{"id": "output", "position": "bottom", "x": 55.03916432328342, "y": 36.00006521998462, "width": 30, "height": 10}],
                    "target": []
                },
                "position": {"x": 212.1047849020086, "y": 93.00563781047852},
                "data": {
                    "label": "8b03771c-2e94-4c2f-b902-8fb8b8e66df2",
                    "node_type": "query",
                    "stream_type": "logs",
                    "org_id": org_id,
                    "query_condition": {
                        "type": "sql",
                        "conditions": None,
                        "sql": "",
                        "promql": None,
                        "promql_condition": None,
                        "aggregation": None,
                        "vrl_function": None,
                        "search_event_type": "DerivedStream"
                    },
                    "trigger_condition": {
                        "period": 15,
                        "operator": "=",
                        "threshold": 0,
                        "frequency": 15,
                        "cron": "",
                        "frequency_type": "minutes",
                        "silence": 0
                    }
                },
                "io_type": "input"
            },
            {
                "id": "cfa476a0-24f3-4ca3-a14f-b56b04729922",
                "type": "output",
                "dimensions": {"width": 250, "height": 42},
                "computedPosition": {"x": 269.2781219770897, "y": 161.6932759313432, "z": 0},
                "handleBounds": {
                    "source": [],
                    "target": [{"id": "input", "position": "top", "x": 110.11741720560622, "y": -4.000002219844093, "width": 30, "height": 10}]
                },
                "position": {"x": 269.2781219770897, "y": 161.6932759313432},
                "data": {
                    "label": "cfa476a0-24f3-4ca3-a14f-b56b04729922",
                    "node_type": "stream",
                    "stream_type": "logs",
                    "stream_name": stream_name,
                    "org_id": org_id
                },
                "io_type": "output"
            }
        ],
        "edges": [
            {
                "id": "e8b03771c-2e94-4c2f-b902-8fb8b8e66df2-cfa476a0-24f3-4ca3-a14f-b56b04729922",
                "type": "custom",
                "source": "8b03771c-2e94-4c2f-b902-8fb8b8e66df2",
                "target": "cfa476a0-24f3-4ca3-a14f-b56b04729922",
                "data": {},
                "markerEnd": {"type": "arrowclosed", "width": 20, "height": 20},
                "style": {"strokeWidth": 2},
                "animated": True
            }
        ],
        "org": org_id,
        "schedule": {
            "enabled": schedule_enabled,
            "frequency": "daily",  # Example frequency, adjust as needed
            "time": "02:00",       # Example time, adjust as needed
        }
    }

    # Create a pipeline
    resp_create_pipeline = session.post(f"{url}api/{org_id}/pipelines", json=pipeline_payload)
    assert resp_create_pipeline.status_code == expected_status, f"Expected status code {expected_status} but got {resp_create_pipeline.status_code}"
    print("Create pipeline response:", resp_create_pipeline.json())
