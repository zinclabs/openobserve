import json
import requests
import pytest
from datetime import datetime, timezone, timedelta


def get_dynamic_timestamps():
    """Generate dynamic timestamps for the query."""
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    return one_min_ago, end_time


def post_query(session, base_url, org_id, query):
    """Helper function to post SQL query and return the response."""
    url = f"{base_url}api/{org_id}/_search?type=logs"
    response = session.post(url, json={"query": query})
    assert (
        response.status_code == 200
    ), f"Expected 200 but got {response.status_code}. Response: {response.content}"
    return response.json()


@pytest.mark.parametrize(
    "sql_query",
    [
        "SELECT * FROM \"stream_pytest_data\" WHERE fuzzy_match(kubernetes_namespace_name, 'ZINC', 4)",
        "SELECT * FROM \"stream_pytest_data\" WHERE fuzzy_match(kubernetes_namespace_name, 'Z', 4)",
        "SELECT * FROM \"stream_pytest_data\" WHERE kubernetes_namespace_name IS NOT NULL AND fuzzy_match(kubernetes_namespace_name, 'Z', 4)",
        "SELECT * FROM \"stream_pytest_data\" WHERE kubernetes_namespace_name LIKE '%zinc%' AND fuzzy_match(kubernetes_namespace_name, 'zinc', 4)",
    ],
)
def test_e2e_fuzzy(create_session, base_url, sql_query):
    """Running E2E tests for various SQL queries."""
    session = create_session
    org_id = "default"
    one_min_ago, end_time = get_dynamic_timestamps()

    query = {
        "sql": sql_query,
        "start_time": one_min_ago,
        "end_time": end_time,
        "from": 0,
        "size": 150,
        "sql_mode": "full",
    }

    response_data = post_query(session, base_url, org_id, query)

    # Additional Assertions on the response data
    assert "hits" in response_data, "Response does not contain 'hits'."
    assert len(response_data["hits"]) > 0, "No logs found in the response."
    