"""MVP 2 Linux native import/inspection/persistence with synthetic security logs.

Run with Xvfb, DBus and tauri-driver on port 4444:
    python3 tests/native_security_formats.py /absolute/path/to/dfir-log-explorer
"""
import base64
import json
import pathlib
import sys
import tempfile
from native_smoke import command, wait_for
from native_cases import location


def main():
    binary = str(pathlib.Path(sys.argv[1]).resolve())
    fixtures = pathlib.Path(__file__).resolve().parent / "fixtures"
    result = command("POST", "/session", {"capabilities": {"alwaysMatch": {"tauri:options": {"application": binary}}}})
    root = "/session/" + result["sessionId"]

    def script(code):
        return command("POST", root + "/execute/sync", {"script": code, "args": []})

    def invoke(name, payload=None):
        result = command("POST", root + "/execute/async", {
            "script": "const done = arguments[arguments.length-1]; window.__TAURI_INTERNALS__.invoke(" + json.dumps(name) + ", " + json.dumps(payload or {}) + ").then(done).catch(e => done({error:String(e)}));", "args": [],
        })
        assert "error" not in result, result
        return result

    def click(selector):
        script("document.querySelector(" + json.dumps(selector) + ").click()")

    def select(selector, value):
        script("const el = document.querySelector(" + json.dumps(selector) + "); el.value = " + json.dumps(value) + "; el.dispatchEvent(new Event('change', {bubbles:true}))")

    def query(**criteria):
        return invoke("query_events", {"query": {"search": "", "onlyWarnings": False, "offset": 0, "limit": 500, **criteria}})

    try:
        wait_for(lambda: script("return !!document.querySelector('.case-bar')"))
        assert not script("return !!document.querySelector('.preview-banner')")
        for mode, fixture, expected in [("syslog", "syslog.log", 5), ("iptables", "iptables.log", 8), ("haproxy", "haproxy.log", 11), ("modsecurity", "modsecurity-audit.log", 14)]:
            select("[aria-label='Parser format']", mode)
            click(".import-actions .primary-button")
            location(fixtures / fixture)
            wait_for(lambda: query()["total"] == expected)
        sources = invoke("case_summary")["sources"]
        assert [s["totalEvents"] for s in sources] == [5, 3, 3, 3]
        assert sources[3]["totalLines"] == 30
        detail = invoke("event_detail", {"id": 12})
        assert (detail["lineNumber"], detail["endLineNumber"], detail["status"]) == (1, 19, 403)
        assert len(detail["jsonFields"]["waf.messages"]) == 2
        assert detail["parseError"] is None
        assert "--abc123-Z--" in detail["raw"]
        assert query(parser="iptables", jsonFilter={"key": "firewall.DPT", "value": "443"})["total"] == 2
        assert query(parser="iptables", hasTimestamp=True, jsonFilter={"key": "firewall.DPT", "value": "443"})["total"] == 1
        # Real UI: reader filter, multiline inspector and scalar attribute action.
        click("[aria-label='Additional filters']")
        select("[aria-label='Reader filter']", "modsecurity")
        wait_for(lambda: script("return document.querySelector('.table-footer')?.textContent.includes('1–3 of 3 events')"))
        click(".table-row")
        wait_for(lambda: script("return document.querySelector('.detail-heading h2')?.textContent === 'Lines 1–19'"))
        assert "Synthetic second rule" in script("return document.querySelector('.detail-panel').textContent")
        click("[aria-label='Filter by attribute waf.action']")
        wait_for(lambda: script("return document.querySelector('.table-footer')?.textContent.includes('1–1 of 1 events')"))
        with tempfile.TemporaryDirectory(prefix="dfir-mvp2-native-") as directory:
            case_path = pathlib.Path(directory) / "security.dfir"
            click(".case-toggle")
            click("[data-case-action=save]")
            location(case_path)
            wait_for(lambda: invoke("case_summary")["path"] == str(case_path))
            if script("return !!document.querySelector(" + json.dumps("[aria-label='Close event details']") + ")"):
                click("[aria-label='Close event details']")
            click(".case-toggle")
            click("[data-case-action=new]")
            wait_for(lambda: len(invoke("case_summary")["sources"]) == 0)
            click(".case-toggle")
            click("[data-case-action=open]")
            location(case_path)
            wait_for(lambda: len(invoke("case_summary")["sources"]) == 4)
            assert query()["total"] == 14
            assert invoke("case_summary")["sources"] == sources
            assert invoke("event_detail", {"id": 12}) == detail
            screenshot = command("GET", root + "/screenshot")
            pathlib.Path("/tmp/dfir-mvp2-native.png").write_bytes(base64.b64decode(screenshot))
            invoke("close_case")
        print("Native MVP 2 passed: four real file-picker imports, multiline provenance, reader/attribute filters, inspector actions and save/new/open persistence.")
    finally:
        command("DELETE", root)


if __name__ == "__main__":
    main()
