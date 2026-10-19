"""Delivery 3 native UI, Tauri commands and real case save/open dialogs.

Run in an isolated Xvfb/DBus session with tauri-driver on port 4444:
    python3 tests/native_investigation.py /absolute/path/to/dfir-log-explorer
Only synthetic evidence and temporary cases are used.
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
    with tempfile.TemporaryDirectory(prefix="dfir-native-investigation-") as directory:
        path = pathlib.Path(directory) / "investigation.dfir"
        session = command("POST", "/session", {"capabilities": {"alwaysMatch": {"tauri:options": {"application": binary}}}})
        root = "/session/" + session["sessionId"]

        def script(code):
            return command("POST", root + "/execute/sync", {"script": code, "args": []})

        def invoke(name, payload=None):
            return command("POST", root + "/execute/async", {
                "script": "const done=arguments[arguments.length-1];window.__TAURI_INTERNALS__.invoke(" + json.dumps(name) + "," + json.dumps(payload or {}) + ").then(done).catch(e=>done({error:String(e)}));", "args": [],
            })

        def click(selector):
            script("document.querySelector(" + json.dumps(selector) + ").click()")

        def fill(selector, value):
            script("const e=document.querySelector(" + json.dumps(selector) + ");Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(e," + json.dumps(value) + ");e.dispatchEvent(new Event('input',{bubbles:true}));")

        def select(selector, value):
            script("const e=document.querySelector(" + json.dumps(selector) + ");e.value=" + json.dumps(value) + ";e.dispatchEvent(new Event('change',{bubbles:true}));")

        def action(text):
            script("[...document.querySelectorAll('button')].find(b=>b.textContent.trim()===" + json.dumps(text) + ").click()")

        def footer(text):
            return script("return document.querySelector('.table-footer')?.textContent.includes(" + json.dumps(text) + ")")

        try:
            wait_for(lambda: script("return !!document.querySelector('.sample-button')"))
            assert not script("return !!document.querySelector('.preview-banner')")
            click(".sample-button")
            wait_for(lambda: footer("1–13 of 13 events"))
            raw = invoke("event_detail", {"id": 1})["raw"]
            fill("[aria-label='Search logs']", "POST")
            wait_for(lambda: footer("1–2 of 2 events"))
            action("Saved searches")
            wait_for(lambda: script("return !document.querySelector('[aria-label=\"Search name\"]').disabled"))
            fill("[aria-label='Search name']", "Login activity")
            action("Save as new")
            wait_for(lambda: isinstance(invoke("saved_searches"), list) and len(invoke("saved_searches")) == 1)
            saved = invoke("saved_searches")[0]
            assert saved["query"]["search"] == "POST"
            action("Close saved searches")
            click(".case-toggle")
            click("[data-case-action=save]")
            location(path)
            wait_for(lambda: invoke("case_summary").get("path") == str(path))
            click(".case-toggle")
            click("[data-case-action=new]")
            wait_for(lambda: invoke("case_summary").get("sources") == [])
            assert invoke("saved_searches") == []
            click(".case-toggle")
            click("[data-case-action=open]")
            location(path)
            wait_for(lambda: invoke("case_summary").get("path") == str(path))
            wait_for(lambda: footer("1–13 of 13 events"))
            action("Saved searches")
            wait_for(lambda: script("return !!document.querySelector('[aria-label=\"Saved search\"] option[value=\"1\"]')"))
            select("[aria-label='Saved search']", "1")
            action("Apply search")
            wait_for(lambda: footer("1–2 of 2 events"))
            assert script("return document.querySelector('[aria-label=\"Search logs\"]').value") == "POST"
            action("Saved searches")
            fill("[aria-label='Search name']", "Renamed login activity")
            action("Update selected")
            wait_for(lambda: invoke("saved_searches")[0]["name"] == "Renamed login activity")
            action("Delete search")
            action("Confirm delete search")
            wait_for(lambda: invoke("saved_searches") == [])
            assert invoke("event_detail", {"id": 1})["raw"] == raw
            assert len(invoke("case_summary")["sources"]) == 1
            action("Close saved searches")
            # Native aggregate results are tied to full queries, not table previews.
            result = invoke("query_events", {"query": {"search": "POST", "limit": 1}})
            assert result["total"] == 2
            assert result["statistics"]["topIps"][0]["count"] == 2
            assert result["statistics"]["topPaths"][0]["filterValue"] == "/login"
            try:
                invoke("put_saved_search", {"id": None, "name": "Missing source", "query": {"sourceId": 999, "limit": 500}})
            except RuntimeError as error:
                assert "unavailable source" in str(error)
            else:
                raise AssertionError("Missing source must be rejected")
            screenshot = command("GET", root + "/screenshot")
            pathlib.Path("/tmp/dfir-delivery3-native.png").write_bytes(base64.b64decode(screenshot))
            invoke("close_case")
            print("Native Delivery 3 passed: create/apply/update/delete through React and real Tauri IPC; temporary-to-saved promotion and real new/open dialogs; correct native aggregates, explicit missing-source rejection and unchanged raw evidence.")
        except Exception:
            try:
                image = command("GET", root + "/screenshot")
                pathlib.Path("/tmp/dfir-delivery3-native-failure.png").write_bytes(base64.b64decode(image))
            except Exception:
                pass
            raise
        finally:
            command("DELETE", root)


if __name__ == "__main__":
    main()
