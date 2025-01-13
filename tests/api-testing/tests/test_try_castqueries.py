
import pytest
from datetime import datetime, timezone, timedelta

# Define test data with different queries and expected response details for cast abd try_cast
test_data = [
    (
        "logs query",
        "SELECT kubernetes_namespace_name from stream_pytest_data LIMIT 1",
        1,
    ),

       (
        "logs cast text query",
        "SELECT cast(kubernetes_namespace_name as Text) as kubernetes_namespace_name from stream_pytest_data LIMIT 1",
        1,
    ),
    (
        "logs try_cast text query",
        "SELECT try_cast(kubernetes_namespace_name as Text) as kubernetes_namespace_name from stream_pytest_data LIMIT 1",
        1,
    ),

       (
        "logs cast VARCHAR query",
        "SELECT cast(kubernetes_namespace_name as VARCHAR) as kubernetes_namespace_name from stream_pytest_data LIMIT 1",
        1,
    ),
    (
        "logs try_cast VARCHAR query",
        "SELECT try_cast(kubernetes_namespace_name as VARCHAR) as kubernetes_namespace_name from stream_pytest_data LIMIT 1",
        1,
    ),

(
    "logs integer comparison query",
    'SELECT * FROM "default" WHERE k8s_container_restart_count = 1',
    1,
),

  (
    "logs integer cast comparison query",
    'SELECT * FROM "default" WHERE cast(k8s_container_restart_count as INT) = 1',
    1,
),  
    
(
    "logs integer try_cast comparison query",
    'SELECT * FROM "default" WHERE try_cast(k8s_container_restart_count as INT) = 1',
    1,
),  
   

]

@pytest.mark.parametrize("test_name, sql_query, expected_size", test_data)
def test_e2e_try_cast_queries(create_session, base_url, test_name, sql_query, expected_size):
    """Parameterized E2E test for various try_cast SQL queries."""
    session = create_session
    url = base_url
    org_id = "default"
    
    # Define time range for the query
    now = datetime.now(timezone.utc)
    end_time = int(now.timestamp() * 1000000)
    one_min_ago = int((now - timedelta(minutes=1)).timestamp() * 1000000)
    
    # Query data
    json_data = {
        "query": {
            "sql": sql_query,
            "start_time": one_min_ago,
            "end_time": end_time,
            "from": 0,
            "size": expected_size,
            "quick_mode": False,
            "sql_mode": "full"
        }
    }
    
    # Send POST request with the session
    response = session.post(f"{url}api/{org_id}/_search?type=logs", json=json_data)
    
    # Assert that the response status is 200
    assert response.status_code == 200, f"{test_name} failed: Expected status 200, got {response.status_code} - {response.content}"
