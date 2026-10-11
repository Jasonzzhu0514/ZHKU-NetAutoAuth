import importlib.util
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("netauth", Path(__file__).resolve().parents[1] / "src" / "netauth.py")
auth = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = auth
spec.loader.exec_module(auth)


class AuthTests(unittest.TestCase):
    def setUp(self):
        self.output = io.StringIO()
        capture = patch('sys.stdout', self.output)
        capture.start()
        self.addCleanup(capture.stop)

    def test_deletion_clears_current_legacy_and_temporary_credentials_only(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            current = root / 'ZHKU-NetAutoAuth' / 'credentials.dat'
            legacy = root / 'SmartZHKU-WindowsDemo' / 'credentials.dat'
            targets = [current, current.with_suffix('.tmp'), legacy, legacy.with_suffix('.tmp')]
            for target in targets:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(b'encrypted-test-data')
            unrelated = root / 'unrelated.txt'
            unrelated.write_text('keep')
            with patch.object(auth, 'config_path', return_value=current):
                auth.delete_credentials()
                self.assertTrue(all(not target.exists() for target in targets))
                self.assertEqual(unrelated.read_text(), 'keep')
                with self.assertRaises(auth.AuthError):
                    auth.load_credentials()
                auth.delete_credentials()  # Already absent is harmless.

    def test_credential_menu_cancellation_does_not_delete(self):
        with patch('builtins.input', side_effect=['2', 'n', '0']), patch.object(auth, 'delete_credentials') as delete:
            auth.credential_menu(auth.OfficePortalClient())
            delete.assert_not_called()

    def test_dormitory_never_uses_office_protocol_or_credentials(self):
        client = auth.create_client('dormitory', 'http://portal.example')
        with patch.object(auth.OfficePortalClient, 'request') as request, patch.object(auth, 'load_credentials') as load:
            for command in ['status', 'login', 'watch', 'setup']:
                with self.subTest(command=command), self.assertRaises(auth.AuthError):
                    auth.run_command(command, client, 10)
            request.assert_not_called()
            load.assert_not_called()
        self.assertEqual(client.portal, auth.DORMITORY_PORTAL)
        self.assertIsNone(client.config_port)

    def test_legacy_region_defaults_to_auto(self):
        with patch.object(auth, 'load_credentials', return_value={'username': 'test'}):
            self.assertIsInstance(auth.create_client(), auth.AutoPortalClient)

    def test_region_detection_checks_office_then_dormitory(self):
        with patch.object(auth, 'probe_portal', return_value=True) as probe:
            self.assertEqual(auth.detect_region(), 'office')
            probe.assert_called_once_with(auth.OFFICE_PORTAL)
        with patch.object(auth, 'probe_portal', side_effect=[False, True]) as probe:
            self.assertEqual(auth.detect_region(), 'dormitory')
            self.assertEqual([call.args[0] for call in probe.call_args_list], [auth.OFFICE_PORTAL, auth.DORMITORY_PORTAL])
        with patch.object(auth, 'probe_portal', side_effect=[False, False]), self.assertRaises(auth.AuthError):
            auth.detect_region()

    def test_auto_detection_never_sends_office_credentials_to_dormitory(self):
        client = auth.AutoPortalClient()
        with patch.object(auth, 'detect_region', return_value='dormitory'), patch.object(client.office, 'login') as login:
            status = client.status()
            self.assertEqual((status.state, status.region), ('unknown', 'dormitory'))
            with self.assertRaises(auth.AuthError):
                client.login({'username': 'test', 'password': 'secret'})
            login.assert_not_called()

    def test_auto_detection_rechecks_when_region_changes(self):
        client = auth.AutoPortalClient()
        with patch.object(auth, 'detect_region', side_effect=['office', 'dormitory']), patch.object(client.office, 'status', return_value=auth.Status('online')):
            self.assertEqual(client.status().region, 'office')
            self.assertEqual(client.status().region, 'dormitory')

    def test_portal_is_user_supplied_and_strips_page_parameters(self):
        self.assertEqual(auth.validate_portal('http://portal.example/a79.htm?token=example'), 'http://portal.example')
        self.assertEqual(auth.validate_portal('https://portal.example:8443/a79.htm'), 'https://portal.example:8443')
        for value in ['portal.example', 'http://user:password@portal.example', 'file:///config', 'http://portal.example:bad']:
            with self.subTest(value=value), self.assertRaises(auth.AuthError):
                auth.validate_portal(value)

    def test_login_query_order_matches_browser(self):
        client = auth.OfficePortalClient()
        with patch.object(client, "request", return_value='campusAuth({"result":1})') as request:
            client.api("/drcom/login", {"DDDDD": "test", "upass": "secret", "0MKKey": "123456"})
            query = request.call_args.args[1]
            self.assertEqual(list(query), ["callback", "DDDDD", "upass", "0MKKey", "jsVersion", "v", "lang"])

    def test_html_error_identifies_endpoint_without_echoing_credentials(self):
        client = auth.OfficePortalClient()
        with patch.object(client, "request", return_value='<html>Error code: 203 Bad request(2) secret</html>'):
            with self.assertRaises(auth.AuthError) as error:
                client.api("/drcom/login", {"upass": "secret"})
            self.assertIn("/drcom/login", str(error.exception))
            self.assertIn("203", str(error.exception))
            self.assertNotIn("secret", str(error.exception))

    def test_jsonp_is_data_not_executable_code(self):
        self.assertEqual(auth.parse_jsonp(' campusAuth({"result":1}); '), {"result": 1})
        for body in ['campusAuth({"result":1});alert(1)', 'other({"result":1})', 'campusAuth([])']:
            with self.subTest(body=body), self.assertRaises(auth.AuthError):
                auth.parse_jsonp(body)

    def test_status_unknown_does_not_mean_offline(self):
        client = auth.OfficePortalClient()
        with patch.object(client, "api", side_effect=auth.AuthError("timeout")):
            self.assertEqual(client.status().state, "unknown")
        with patch.object(client, "api", return_value={"result": "unexpected"}):
            self.assertEqual(client.status().state, "unknown")
        for result, state in [(0, "offline"), (1, "online"), ("1", "online")]:
            with patch.object(client, "api", return_value={"result": result, "v4ip": "10.1.2.3", "ss5": "10.1.2.3", "olmac": "112233445566", "ss4": "aabbccddeeff", "cvid": 1158, "pvid": 0, "uid": "test"}):
                status = client.status()
                self.assertEqual(status.state, state)
                self.assertEqual((status.v4ip, status.ss5, status.online_mac, status.ss4, status.cvid, status.pvid, status.account), ("10.1.2.3", "10.1.2.3", "112233445566", "aabbccddeeff", "1158", "0", "test"))

    def test_online_skips_credentials_and_login(self):
        client = auth.OfficePortalClient()
        with patch.object(client, "status", return_value=auth.Status("online")), patch.object(client, "api") as api:
            self.assertEqual(client.login({}).state, "online")
            api.assert_not_called()
        with patch.object(client, "status", return_value=auth.Status("online")), patch.object(auth, "load_credentials") as load:
            self.assertEqual(auth.run_command("login", client, 10), 0)
            load.assert_not_called()

    def test_unknown_never_submits_password(self):
        client = auth.OfficePortalClient()
        with patch.object(client, "status", return_value=auth.Status("unknown", detail="timeout")), patch.object(client, "api") as api:
            with self.assertRaises(auth.AuthError):
                client.login({"username": "test", "password": "secret"})
            api.assert_not_called()

    def test_login_matches_observed_protocol_and_verifies_online(self):
        client = auth.OfficePortalClient()
        with patch.object(client, "status", side_effect=[auth.Status("offline"), auth.Status("online")]), patch.object(client, "request", return_value='vlanid="1158";'), patch.object(client, "api", side_effect=[{"data": {"login_method": "0"}}, {"result": 1}]) as api:
            result = client.login({"username": "test", "password": "a&b+c", "suffix": "@dx"})
            self.assertEqual(result.state, "online")
            path, params = api.call_args.args
            self.assertEqual(path, "/drcom/login")
            self.assertEqual(params["DDDDD"], "test")
            self.assertEqual(params["upass"], "a&b+c")
            self.assertEqual(params["R2"], "")
            self.assertEqual(params["para"], "00")

    def test_success_reply_without_online_state_is_failure(self):
        client = auth.OfficePortalClient()
        with patch.object(client, "status", return_value=auth.Status("offline")), patch.object(client, "request", return_value=""), patch.object(client, "api", side_effect=[{"data": {"login_method": 0}}, {"result": 1, "msg": "secret"}]), patch.object(auth.time, "sleep"):
            with self.assertRaises(auth.AuthError) as error:
                client.login({"username": "test", "password": "secret"})
            self.assertNotIn("secret", str(error.exception))

    @unittest.skipUnless(auth.os.name == "nt", "Windows DPAPI")
    def test_dpapi_round_trip(self):
        payload = 'test password 密码'.encode("utf-8")
        encrypted = auth.dpapi(payload)
        self.assertNotIn(payload, encrypted)
        self.assertEqual(auth.dpapi(encrypted, decrypt=True), payload)


if __name__ == "__main__":
    unittest.main()
