import base64
import os
from pathlib import Path
import random
import requests
import pytest

# Constants for WebSocket URL and user credentials
ZO_BASE_URL_SC = os.environ.get("ZO_BASE_URL_SC")  # Use environment variable
ZO_BASE_URL = os.environ.get("ZO_BASE_URL")  # Use environment variable
ZO_ROOT_USER_EMAIL = os.environ.get("ZO_ROOT_USER_EMAIL")  # Use environment variable
ZO_ROOT_USER_PASSWORD = os.environ.get("ZO_ROOT_USER_PASSWORD")  # Use environment variable
org_id = "default"

# Define the email_id for the service account
email_id = f"em_{random.randint(1000, 9999)}@gmail.com"  # Make the name unique
action_name = f"ac_{random.randint(1000, 9999)}"  # Make the name unique

root_dir = Path(__file__).parent.parent.parent

def base_url():
    """Provide the base URL for the API of Super Cluster."""
    return ZO_BASE_URL

def test_get_actions(create_session, base_url):
    """Running an E2E test for getting all the actions list."""
    session = create_session
    resp_get_allactions = session.get(f"{base_url}api/{org_id}/actions")

    print("actions", resp_get_allactions.content)
    assert (
        resp_get_allactions.status_code == 200
    ), f"Expected 200, but got {resp_get_allactions.status_code} {resp_get_allactions.content}"

def test_create_serviceaccount(create_session, base_url):
    """Running an E2E test for creating and deleting a service account."""
    session = create_session
    # Create a service account
    payload_serviceaccounts = {
        "email": email_id,
        "organization": org_id,
        "first_name": "Action",
        "last_name": "Services",
    }

    resp_create_serviceaccount = session.post(
        f"{base_url}api/{org_id}/service_accounts", json=payload_serviceaccounts
    )

    print("Service Account Created", resp_create_serviceaccount.content)

    assert (
        resp_create_serviceaccount.status_code == 200
    ), f"Expected 200, but got {resp_create_serviceaccount.status_code} {resp_create_serviceaccount.content}"

    # Delete the service account
    resp_delete_serviceaccount = session.delete(
        f"{base_url}api/{org_id}/service_accounts/{email_id}"
    )

    print("Service Account Deleted", resp_delete_serviceaccount.content)

    assert (
        resp_delete_serviceaccount.status_code == 200
    ), f"Expected 200, but got {resp_delete_serviceaccount.status_code} {resp_delete_serviceaccount.content}"

# Define the headers for the request
HEADERS = {
    'accept': 'application/json, text/plain, */*',
    'accept-language': 'en-US,en;q=0.9',
    'cache-control': 'no-cache',
    'content-type': 'multipart/form-data',  # requests will handle boundary automatically
    'cookie': f"auth_tokens={{\"access_token\":\"Basic {base64.b64encode((ZO_ROOT_USER_EMAIL + ':' + ZO_ROOT_USER_PASSWORD).encode()).decode()}\",\"refresh_token\":\"\"}}",
    'origin': base_url(),
    'pragma': 'no-cache',
    'priority': 'u=1, i',
    'referer': f"{base_url()}/web/action-scripts?action=add&org_identifier={org_id}",
    'sec-ch-ua': '"Not A(Brand";v="8", "Chromium";v="132", "Google Chrome";v="132"',
    'sec-ch-ua-mobile': '?0',
    'sec-ch-ua-platform': '"macOS"',
    'sec-fetch-dest': 'empty',
    'sec-fetch-mode': 'cors',
    'sec-fetch-site': 'same-origin',
    'user-agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36'
}

@pytest.fixture
def upload_file():
    """Fixture to handle the file upload."""
    # Define the URL for the API request
    URL_ACTION = f"{base_url()}api/{org_id}/actions/upload"

    # Print the URL for debugging
    print(f"Constructed URL: {URL_ACTION}")

    # Open the ZIP file and prepare the multipart/form-data request
    zip_file_path = root_dir / "test-data/test1234_py.zip"
    
    with open(zip_file_path, 'rb') as f:
        DATA = {
            'name': action_name,
            'description': 'action description',
            'execution_details': 'Once',
            'service_account': email_id,
            'owner': ZO_ROOT_USER_EMAIL,
            'environment_variables': '{}',
            'file': ('test1234_py.zip', f, 'application/zip'),
            'filename': 'test1234_py.zip'
        }

        # Perform the API request
        response = requests.post(URL_ACTION, headers=HEADERS, files=DATA)

        # Debugging response content if not successful
    if response.status_code != 200:
        print(f"Upload Error: {response.status_code}, Content: {response.content}")
    
    return response

def test_upload_file(upload_file: requests.Response):
    """Test the file upload API."""
    response = upload_file
    
    # Check if the response is successful
    assert response.status_code == 200, f"Expected 200, got {response.status_code}"
    
    # Optionally, check the response content
    response_json = response.json()
    assert 'success' in response_json, "Response does not contain 'success' key"

if __name__ == "__main__":
    pytest.main()
