"""ZHKU-NetAutoAuth: Windows campus network authentication."""

import argparse
import ctypes
from ctypes import wintypes
from dataclasses import dataclass
from datetime import datetime
import getpass
import gzip
import json
import os
from pathlib import Path
import re
import time
import urllib.error
import urllib.parse
import urllib.request


CALLBACK = "campusAuth"
VERSION = "1.0.0"
# Campus infrastructure defaults, not personal device or account information.
OFFICE_PORTAL = "http://10.255.255.254"
OFFICE_CONFIG_PORT = 801
DORMITORY_PORTAL = "http://192.16.99.5"


class AuthError(Exception):
    pass


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        # Credentials in the portal's GET request must stay on the original host.
        return None


def parse_jsonp(text):
    text = text.strip()
    if text.startswith("{"):
        payload = text
    else:
        match = re.fullmatch(re.escape(CALLBACK) + r"\s*\((.*)\)\s*;?", text, re.S)
        if not match:
            raise AuthError("认证接口返回了无法识别的内容。")
        payload = match.group(1)
    try:
        value = json.loads(payload)
    except ValueError:
        raise AuthError("认证接口返回了无效的 JSON。") from None
    if not isinstance(value, dict):
        raise AuthError("认证接口返回的数据格式不正确。")
    return value


@dataclass
class Status:
    state: str
    ip: str = ""
    account: str = ""
    detail: str = ""
    v4ip: str = ""
    ss5: str = ""
    online_mac: str = ""
    ss4: str = ""
    cvid: str = ""
    pvid: str = ""
    region: str = ""


def validate_portal(value):
    try:
        parts = urllib.parse.urlsplit(value.strip())
        if parts.scheme not in ("http", "https") or not parts.hostname or parts.username or parts.password:
            raise ValueError()
        parts.port  # Validate port syntax and range.
        return urllib.parse.urlunsplit((parts.scheme, parts.netloc, "", "", ""))
    except ValueError:
        raise AuthError("请输入有效的认证页面地址（http:// 或 https://，不能包含账号密码）。") from None


class OfficePortalClient:
    region = "office"

    def __init__(self, portal=None, config_port=None, timeout=8):
        self.timeout = timeout
        self.portal = validate_portal(portal or OFFICE_PORTAL)
        self.config_port = config_port
        # Do not send local campus requests through an environment/system proxy.
        self.opener = urllib.request.build_opener(
            urllib.request.ProxyHandler({}), NoRedirect()
        )

    def request(self, path, params=None, port=None):
        portal = self.portal
        parts = urllib.parse.urlsplit(portal)
        if port is not None:
            host = f"[{parts.hostname}]" if ":" in parts.hostname else parts.hostname
            origin = urllib.parse.urlunsplit((parts.scheme, f"{host}:{port}", "", "", ""))
        else:
            origin = portal
        url = origin + path
        if params is not None:
            url += "?" + urllib.parse.urlencode(params)
        req = urllib.request.Request(url, headers={
            "User-Agent": "Mozilla/5.0 ZHKU-NetAutoAuth/1.0.0",
            "Referer": portal + "/a79.htm",
            "Accept": "*/*",
            "Cache-Control": "no-cache",
        })
        try:
            with self.opener.open(req, timeout=self.timeout) as response:
                body = response.read(1024 * 1024)
                if body.startswith(b"\x1f\x8b"):
                    body = gzip.decompress(body)
                charset = response.headers.get_content_charset()
                if not charset:
                    charset = "gb18030" if "text/html" in response.headers.get("Content-Type", "") else "utf-8"
                return body.decode(charset, errors="replace")
        except urllib.error.HTTPError as exc:
            raise AuthError(f"认证服务器返回 HTTP {exc.code}。") from None
        except (OSError, urllib.error.URLError, ValueError, LookupError):
            # Never print the exception URL: it may contain the password.
            raise AuthError("认证服务器不可达或响应异常，请检查网线、校园网和代理/VPN路由。") from None

    def api(self, path, params=None, port=None):
        # Dr.COM's login parser is order-sensitive: match a41.js formatParams.
        # In particular DDDDD/upass must precede jsVersion/v/lang.
        query = {"callback": CALLBACK}
        query.update(params or {})
        query["jsVersion"] = "4.X"
        query["v"] = time.time_ns() % 10000 + 500
        query["lang"] = "zh"
        body = self.request(path, query, port)
        try:
            return parse_jsonp(body)
        except AuthError as exc:
            # Extract only a numeric error code; never echo the raw response.
            error = re.search(r"Error code:\s*(\d{1,6})\b", body, re.I)
            detail = f"服务器错误码 {error.group(1)}" if error else str(exc)
            raise AuthError(f"接口 {path}：{detail}") from None

    def status(self):
        try:
            data = self.api("/drcom/chkstatus")
        except AuthError as exc:
            return Status("unknown", detail=str(exc))
        result = str(data.get("result", ""))
        if result not in ("0", "1"):
            return Status("unknown", detail="在线接口没有返回有效的 result（0/1）。")
        def field(name):
            value = data.get(name)
            return "" if value is None else str(value)

        return Status(
            state="online" if result == "1" else "offline",
            ip=field("v4ip") or field("v46ip") or field("ss5"),
            account=field("uid"),
            v4ip=field("v4ip"), ss5=field("ss5"),
            online_mac=field("olmac"), ss4=field("ss4"),
            cvid=field("cvid"), pvid=field("pvid"),
        )

    def login(self, credentials):
        current = self.status()
        if current.state == "online":
            return current
        if current.state == "unknown":
            raise AuthError(current.detail + " 本轮未提交登录。")

        page = self.request("/a79.htm")
        vlan = re.search(r'vlanid\s*=\s*[\'"]([^\'"]*)', page)
        config = self.api("/eportal/portal/page/loadConfig", {
            "program_index": "", "wlan_vlan_id": vlan.group(1) if vlan else "",
            "gw_id": "000000000000",
        }, port=self.config_port or credentials.get("config_port", OFFICE_CONFIG_PORT)).get("data")
        if not isinstance(config, dict) or str(config.get("login_method")) != "0":
            raise AuthError("页面认证方式已变化；当前版本仅支持 Dr.COM login_method=0。")
        if str(config.get("en_md5", "0")) not in ("0", "", "None"):
            raise AuthError("服务器启用了 MD5 登录，需要重新适配页面参数。")

        account = credentials["username"]
        # a40.js default_login uses this.account, without the ',0,' prefix.
        reply = self.api("/drcom/login", {
            "DDDDD": account, "upass": credentials["password"], "0MKKey": "123456",
            "R1": "0", "R2": "", "R3": "0", "R6": "0", "para": "00",
            "v6ip": "", "terminal_type": "1",
        })
        # A positive login response alone is insufficient; recheck actual online state.
        for attempt in range(3):
            if attempt:
                time.sleep(1)
            checked = self.status()
            if checked.state == "online":
                return checked
        code = reply.get("ret_code", reply.get("msga", reply.get("result", "未知")))
        # Display only a bounded code, not raw server messages that can echo secrets.
        code = re.sub(r"[^0-9A-Za-z_.-]", "", str(code))[:24] or "未知"
        raise AuthError(f"认证后未确认在线（服务器代码：{code}）。请检查账号和密码，或查看浏览器登录响应。")


class DormitoryPortalClient:
    """Known portal address; dormitory authentication protocol is still reserved."""
    region = "dormitory"
    portal = DORMITORY_PORTAL
    config_port = None

    def status(self):
        raise AuthError("已识别宿舍区认证地址，但宿舍区认证接口和协议暂未适配，不提交登录。")

    def login(self, credentials):
        return self.status()


def probe_portal(portal):
    """Check HTTP reachability without sending credentials or using a proxy."""
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    try:
        with opener.open(portal, timeout=2) as response:
            response.read(64)
            return 200 <= response.status < 400
    except urllib.error.HTTPError as exc:
        return 300 <= exc.code < 400
    except (OSError, urllib.error.URLError, ValueError):
        return False


def detect_region(office_portal=OFFICE_PORTAL):
    if probe_portal(office_portal):
        return "office"
    if probe_portal(DORMITORY_PORTAL):
        return "dormitory"
    raise AuthError("两个区域的认证地址均不可达，请先接入白云校区校园网。")


class AutoPortalClient:
    region = "auto"

    def __init__(self, portal=None, config_port=None):
        self.office = OfficePortalClient(portal, config_port)
        self.portal = self.office.portal
        self.config_port = config_port

    def select(self):
        region = detect_region(self.portal)
        return self.office if region == "office" else DormitoryPortalClient()

    def status(self):
        selected = None
        try:
            selected = self.select()
            result = selected.status()
        except AuthError as exc:
            result = Status("unknown", detail=str(exc))
        if selected is not None:
            result.region = selected.region
        return result

    def login(self, credentials):
        selected = self.select()
        result = selected.login(credentials)
        result.region = selected.region
        return result


def create_client(region=None, portal=None, config_port=None):
    explicit_region = region
    settings = {}
    if region != "dormitory":
        try:
            settings = load_credentials()
        except AuthError:
            pass
    # Automatic detection is the default, including for older office-only configs.
    region = region or "auto"
    if region == "auto":
        client = AutoPortalClient(portal or settings.get("portal"),
                                  config_port or settings.get("config_port"))
    elif region == "office":
        client = OfficePortalClient(portal or settings.get("portal"),
                                   config_port or settings.get("config_port"))
    elif region == "dormitory":
        client = DormitoryPortalClient()
    else:
        raise AuthError("未知的认证区域，请重新配置。")
    client.explicit_region = explicit_region
    return client


class DataBlob(ctypes.Structure):
    _fields_ = [("cbData", wintypes.DWORD), ("pbData", ctypes.POINTER(ctypes.c_ubyte))]


def dpapi(data, decrypt=False):
    if os.name != "nt":
        raise AuthError("凭据加密功能仅支持 Windows。")
    buf = (ctypes.c_ubyte * len(data)).from_buffer_copy(data)
    source = DataBlob(len(data), buf)
    output = DataBlob()
    crypt = ctypes.WinDLL("crypt32", use_last_error=True)
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.LocalFree.argtypes = [ctypes.c_void_p]
    kernel.LocalFree.restype = ctypes.c_void_p
    if decrypt:
        fn = crypt.CryptUnprotectData
        fn.argtypes = [ctypes.POINTER(DataBlob), ctypes.c_void_p,
                       ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p,
                       wintypes.DWORD, ctypes.POINTER(DataBlob)]
        description = None
    else:
        fn = crypt.CryptProtectData
        fn.argtypes = [ctypes.POINTER(DataBlob), wintypes.LPCWSTR,
                       ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p,
                       wintypes.DWORD, ctypes.POINTER(DataBlob)]
        description = "ZHKU-NetAutoAuth"
    fn.restype = wintypes.BOOL
    if not fn(ctypes.byref(source), description, None, None, None, 1, ctypes.byref(output)):
        raise AuthError("Windows DPAPI 操作失败；配置必须由保存它的 Windows 用户使用。")
    try:
        return ctypes.string_at(output.pbData, output.cbData)
    finally:
        kernel.LocalFree(ctypes.cast(output.pbData, ctypes.c_void_p))


def config_path():
    return Path(os.environ.get("LOCALAPPDATA", str(Path.home()))) / "ZHKU-NetAutoAuth" / "credentials.dat"


def load_credentials():
    path = config_path()
    legacy = path.parent.parent / "SmartZHKU-WindowsDemo" / "credentials.dat"
    source = path if path.exists() else legacy
    if not source.exists():
        raise AuthError("尚未配置账号。请先选菜单 2，或运行 setup。")
    try:
        encrypted = source.read_bytes()
        value = json.loads(dpapi(encrypted, decrypt=True).decode("utf-8"))
        if not isinstance(value, dict) or not value.get("username") or not value.get("password"):
            raise ValueError()
        if not all(isinstance(value.get(k), str) for k in ("username", "password")):
            raise ValueError()
        if source != path:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(encrypted)
        # Accept old encrypted configs, but discard the former carrier suffix.
        return {"username": value["username"], "password": value["password"],
                "portal": value.get("portal") or OFFICE_PORTAL,
                "config_port": value.get("config_port") or OFFICE_CONFIG_PORT,
                "region": value.get("region", "office")}
    except (OSError, ValueError, UnicodeError):
        raise AuthError("凭据文件损坏或不可读，请重新运行 setup。") from None


def setup(portal=None, config_port=None, region=None):
    if region is None:
        print("认证模式：1 自动检测（推荐） / 2 办公区 / 3 宿舍区（暂未适配）")
        choice = input("请选择 [默认 1]：").strip() or "1"
        region = {"1": "auto", "2": "office", "3": "dormitory"}.get(choice)
    if region == "dormitory":
        raise AuthError("宿舍区地址已预置，但认证协议暂未适配，不保存配置或提交认证。")
    if region not in ("office", "auto"):
        raise AuthError("无效的认证区域选择。")
    portal = validate_portal(portal or OFFICE_PORTAL)
    config_port = config_port or OFFICE_CONFIG_PORT
    if not 1 <= config_port <= 65535:
        raise AuthError("配置接口端口必须在 1～65535 之间。")
    username = input("校园网账号（保留浏览器登录时使用的完整格式）：").strip()
    password = getpass.getpass("校园网密码（输入不显示）：")
    if not username or not password:
        raise AuthError("账号和密码不能为空。")
    data = {"username": username, "password": password, "portal": portal,
            "config_port": config_port, "region": region}
    encrypted = dpapi(json.dumps(data, ensure_ascii=False).encode("utf-8"))
    path = config_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_bytes(encrypted)
    temporary.replace(path)
    print(f"配置已加密保存：{path}")


def delete_credentials():
    path = config_path()
    legacy = path.parent.parent / "SmartZHKU-WindowsDemo" / "credentials.dat"
    deleted = 0
    failed = False
    for target in (path, path.with_suffix(".tmp"), legacy, legacy.with_suffix(".tmp")):
        try:
            target.unlink()
            deleted += 1
        except FileNotFoundError:
            pass
        except OSError:
            failed = True
    if failed:
        raise AuthError("部分凭据文件无法删除，请检查文件权限或占用情况后重试。")
    if deleted:
        print("已删除正式版及旧版的本地凭据。再次自动认证前需要重新配置；当前校园网会话不会被注销。")
    else:
        print("没有已保存的本地凭据。")


def credential_menu(client):
    while True:
        print("\n配置账号密码\n1 新增或修改配置\n2 删除已保存凭据\n0 返回主菜单")
        choice = input("请选择：").strip()
        try:
            if choice == "0":
                return
            if choice == "1":
                setup(client.portal, client.config_port, getattr(client, "explicit_region", None))
            elif choice == "2":
                print("将删除本机当前用户保存的正式版、旧版凭据及临时配置。")
                confirmed = input("确认删除？[y/N]：").strip().lower()
                if confirmed in ("y", "yes"):
                    delete_credentials()
                else:
                    print("已取消删除。")
            else:
                print("请输入 0～2。")
        except AuthError as exc:
            print(f"错误：{exc}")


def show_status(status):
    stamp = datetime.now().strftime("%H:%M:%S")
    if status.region:
        stamp += "][" + {"office": "办公区", "dormitory": "宿舍区"}.get(status.region, status.region)
    if status.state == "online":
        print(f"[{stamp}] 已通过校园网认证")
        print(f"  IPv4 (v4ip)：{status.v4ip or '未返回'}")
        print(f"  IPv4 (ss5) ：{status.ss5 or '未返回'}")
        print(f"  在线设备 MAC (olmac)：{status.online_mac or '未返回'}")
        print(f"  服务器原始字段 (ss4) ：{status.ss4 or '未返回'}")
        print(f"  服务器原始字段 (cvid)：{status.cvid or '未返回'}")
        print(f"  服务器原始字段 (pvid)：{status.pvid or '未返回'}")
        print(f"  账号 (uid) ：{status.account or '未返回'}")
    elif status.state == "offline":
        print(f"[{stamp}] 尚未认证；IP：{status.ip or '未知'}")
    else:
        print(f"[{stamp}] 状态未知：{status.detail}")


def watch(client, interval):
    credentials = load_credentials()
    delay = interval
    print(f"开始守护，每 {interval:g} 秒检测。按 Ctrl+C 停止；停止不会注销校园网。")
    while True:
        current = client.status()
        show_status(current)
        if current.state == "offline":
            try:
                show_status(client.login(credentials))
                delay = interval
            except AuthError as exc:
                delay = min(max(delay * 2, 30), 300)
                print(f"{exc} {delay:g} 秒后重试。")
        elif current.state == "online":
            delay = interval
        else:
            delay = min(max(delay * 2, 30), 300)
            print(f"暂不提交认证，{delay:g} 秒后重新检查。")
        time.sleep(delay)


def run_command(command, client, interval):
    if command == "setup":
        setup(client.portal, client.config_port, getattr(client, "explicit_region", None))
    elif command == "status":
        result = client.status()
        show_status(result)
        return {"online": 0, "offline": 1, "unknown": 2}[result.state]
    elif command == "login":
        # Being online must not require saved credentials or trigger another login.
        result = client.status()
        if result.state == "unknown":
            raise AuthError(result.detail)
        if result.state == "offline":
            result = client.login(load_credentials())
        show_status(result)
    elif command == "watch":
        if client.region == "dormitory":
            client.status()  # Fail immediately instead of entering a retry loop.
        watch(client, interval)
    return 0


def main():
    parser = argparse.ArgumentParser(description="ZHKU-NetAutoAuth：仲恺校园网自动认证")
    parser.add_argument("command", nargs="?", choices=["setup", "status", "login", "watch"])
    parser.add_argument("--version", action="version", version=f"ZHKU-NetAutoAuth {VERSION}")
    parser.add_argument("--region", choices=["auto", "office", "dormitory"], help="默认自动检测；可指定办公区或宿舍区（认证暂未适配）")
    parser.add_argument("--portal", help="覆盖办公区认证地址；默认使用已保存地址或预置服务器")
    parser.add_argument("--config-port", type=int, help="Dr.COM 配置接口端口，默认使用保存的配置")
    parser.add_argument("--interval", type=float, default=10, help="守护检查间隔，至少 5 秒")
    args = parser.parse_args()
    if not 5 <= args.interval <= 3600:
        parser.error("--interval 必须在 5～3600 秒之间")
    if args.config_port is not None and not 1 <= args.config_port <= 65535:
        parser.error("--config-port 必须在 1～65535 之间")
    try:
        client = create_client(args.region, args.portal, args.config_port)
        if args.command:
            return run_command(args.command, client, args.interval)
        while True:
            print(f"\nZHKU-NetAutoAuth v{VERSION}（Windows）")
            try:
                office_portal = client.portal if client.region != "dormitory" else OFFICE_PORTAL
                location = {"office": "办公区", "dormitory": "宿舍区（认证暂未适配）"}[detect_region(office_portal)]
            except AuthError:
                location = "未检测到校园网"
            print("所在区域：" + location)
            print("1 查看认证状态\n2 配置账号密码\n3 立即认证\n4 持续守护\n0 退出")
            choice = input("请选择：").strip()
            if choice == "0":
                return 0
            command = {"1": "status", "2": "setup", "3": "login", "4": "watch"}.get(choice)
            if command is None:
                print("请输入 0～4。")
                continue
            try:
                if command == "setup":
                    credential_menu(client)
                else:
                    run_command(command, client, args.interval)
            except AuthError as exc:
                print(f"错误：{exc}")
            except KeyboardInterrupt:
                print("\n已停止守护。")
    except (KeyboardInterrupt, EOFError):
        print("\n已退出。")
        return 0
    except AuthError as exc:
        print(f"错误：{exc}")
        return 2
    except OSError:
        print("本地文件读写失败，请检查配置目录权限。")
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
