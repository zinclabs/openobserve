def test_e2e_users(create_session, base_url):
    """Running an E2E test for get all the users list."""

    session = create_session
    url = base_url
    org_id = "default"

    resp_get_allusers = session.get(f"{url}api/{org_id}/users")

    print(resp_get_allusers.content)
    assert (
        resp_get_allusers.status_code == 200
    ), f"Get all functions list 200, but got {resp_get_allusers.status_code} {resp_get_allusers.content}"


def test_e2e_createusers(create_session, base_url):
    """Running an E2E test for new user creation. This user will be part of default org."""

    session = create_session
    url = base_url
    org_id = "default"
    payload = {
        "organization": "e2e",
        "email": "pytests@gmail.com",
        "password": "Test@029w0",
        "first_name": "Nehapd",
        "last_name": "p",
        "role": "admin",
    }

    resp_create_users = session.post(f"{url}api/{org_id}/users", json=payload)

    print(resp_create_users.content)
    assert (
        resp_create_users.status_code == 200
    ), f"Creating a user 200, but got {resp_create_users.status_code} {resp_create_users.content}"
    resp_delete_users = session.delete(f"{url}api/{org_id}/users/pytests@gmail.com")
    assert (
        resp_delete_users.status_code == 200
    ), f"Deleting this user, but got {resp_delete_users.status_code} {resp_delete_users.content}"


# def test_e2e_createusersasmember(create_session,base_url):
#     """Running an E2E test for new user creation."""

#     session = create_session
#     url = base_url
#     org_id = "default"
#     payload = {
#     "organization": "default",
#     "email": "pytests@gmail.com",
#     "password": "Test@029w0",
#     "first_name": "Nehapd",
#     "last_name": "p",
#     "role": "member"
#     }

#     resp_create_users = session.post(f"{url}api/{org_id}/users",json=payload)

#     print(resp_create_users.content)
#     assert resp_create_users.status_code == 200, f"Creating a user 200, but got {resp_create_users.status_code} {resp_create_users.content}"
#     resp_delete_users = session.delete(f"{url}api/{org_id}/users/pytests@gmail.com")
#     assert resp_delete_users.status_code == 200, f"Deleting this user, but got {resp_delete_users.status_code} {resp_delete_users.content}"


def test_e2e_invalidrole(create_session, base_url):
    """Running an E2E test for invalid role added."""

    session = create_session
    url = base_url
    org_id = "default"
    payload = {
        "organization": "e2e",
        "email": "pytests@gmail.com",
        "password": "T",
        "first_name": "Nehapd",
        "last_name": "p",
        "role": "m",
    }

    resp_create_users = session.post(f"{url}api/{org_id}/users", json=payload)

    print(resp_create_users.content)
    assert (
        resp_create_users.status_code == 400
    ), f"Invalid role added, but got {resp_create_users.status_code} {resp_create_users.content}"


def test_e2e_invalidorganization(create_session, base_url):
    """Running an E2E test for invalid org added while user creation."""

    session = create_session
    url = base_url
    org_id = "default_invalid"
    payload = {
        "email": "pytests@gmail.com",
        "password": "T",
        "first_name": "Nehapd",
        "last_name": "p",
        "role": "admin",
    }

    resp_create_users = session.post(f"{url}api/{org_id}/users", json=payload)

    print(resp_create_users.content)
    assert (
        resp_create_users.status_code == 404
    ), f"Invalid org added, but got {resp_create_users.status_code} {resp_create_users.content}"
    resp_delete_users = session.delete(f"{url}api/{org_id}/users/pytests@gmail.com")
    assert (
        resp_delete_users.status_code == 404
    ), f"Deleting this user, but got {resp_delete_users.status_code} {resp_delete_users.content}"


def test_e2e_addinvalidusertoOrg(create_session, base_url):
    """Running an E2E test for invalid user added to org."""

    session = create_session
    url = base_url
    org_id = "default"
    payload = {
        "organization": "e2e",
        "email": "py@gmail.com",
        "password": "T",
        "first_name": "Nehapd",
        "last_name": "p",
        "role": "admin",
    }

    resp_create_users = session.post(
        f"{url}api/{org_id}/users/pynew@gmail.com", json=payload
    )

    print(resp_create_users.content)
    assert (
        resp_create_users.status_code == 422
    ), f"Invalid user returning 422, but got {resp_create_users.status_code} {resp_create_users.content}"


def test_e2e_updateuser(create_session, base_url):
    """Running an E2E test for adding, updating & deleting user ."""

    session = create_session
    url = base_url
    org_id = "default"
    payload = {
        "organization": "e2e",
        "email": "py@gmail.com",
        "password": "T",
        "first_name": "Nehapd",
        "last_name": "p",
        "role": "admin",
    }

    resp_create_users = session.post(f"{url}api/{org_id}/users", json=payload)

    print(resp_create_users.content)
    assert (
        resp_create_users.status_code == 200
    ), f"Creating a user 200, but got {resp_create_users.status_code} {resp_create_users.content}"

    payload = {
        "organization": "e2e",
        "email": "py@gmail.com",
        "password": "T",
        "first_name": "Editedname",
        "last_name": "p",
        "role": "admin",
    }
    resp_update_users = session.put(
        f"{url}api/{org_id}/users/py@gmail.com", json=payload
    )
    assert (
        resp_update_users.status_code == 200
    ), f"Update this user 200, but got {resp_update_users.status_code} {resp_update_users.content}"
    resp_delete_users = session.delete(f"{url}api/{org_id}/users/py@gmail.com")
    assert (
        resp_delete_users.status_code == 200
    ), f"Deleting this user, but got {resp_delete_users.status_code} {resp_delete_users.content}"


def test_e2e_updateusernotfound(create_session, base_url):
    """Running an E2E test for user not found for updating values."""

    session = create_session
    url = base_url
    org_id = "default"
    payload = {
        "organization": "e2e",
        "email": "py@gmail.com",
        "password": "T",
        "first_name": "Nehapd",
        "last_name": "p",
        "role": "admin",
    }

    resp_create_users = session.post(f"{url}api/{org_id}/users", json=payload)

    print(resp_create_users.content)
    assert (
        resp_create_users.status_code == 200
    ), f"Creating a user 200, but got {resp_create_users.status_code} {resp_create_users.content}"

    payload = {
        "organization": "default",
        "email": "py@gmail.com",
        "password": "T",
        "first_name": "Editedname",
        "last_name": "p",
        "role": "admin",
    }
    resp_delete_users = session.delete(f"{url}api/{org_id}/users/py@gmail.com")
    assert (
        resp_delete_users.status_code == 200
    ), f"Deleting this user, but got {resp_delete_users.status_code} {resp_delete_users.content}"
    resp_update_users = session.put(
        f"{url}api/{org_id}/users/py@gmail.com", json=payload
    )
    assert (
        resp_update_users.status_code == 404
    ), f"Update this user 400, but got {resp_update_users.status_code} {resp_update_users.content}"


def test_e2e_updateuserexists(create_session, base_url):
    """Running an E2E test for user already exists."""

    session = create_session
    url = base_url
    org_id = "default"
    payload = {
        "organization": "default",
        "email": "py@gmail.com",
        "password": "T",
        "first_name": "Nehapd",
        "last_name": "p",
        "role": "admin",
    }

    resp_create_users = session.post(f"{url}api/{org_id}/users", json=payload)

    print(resp_create_users.content)
    assert (
        resp_create_users.status_code == 200
    ), f"Creating a user 200, but got {resp_create_users.status_code} {resp_create_users.content}"

    payload = {
        "organization": "default",
        "email": "py@gmail.com",
        "password": "T",
        "first_name": "Nehapd",
        "last_name": "p",
        "role": "admin",
    }
    print(resp_create_users.content)
    resp_create_users = session.post(f"{url}api/{org_id}/users", json=payload)
    assert (
        resp_create_users.status_code == 400
    ), f"User already exists 400, but got {resp_create_users.status_code} {resp_create_users.content}"
    resp_delete_users = session.delete(f"{url}api/{org_id}/users/py@gmail.com")
    assert (
        resp_delete_users.status_code == 200
    ), f"Deleting this user 200, but got {resp_delete_users.status_code} {resp_delete_users.content}"
