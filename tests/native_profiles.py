"""Delivery 4 profiles through React/Tauri, real file dialogs and persistent cases.
Run with tauri-driver in an isolated Xvfb/DBus session. Synthetic evidence only.
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
    source = pathlib.Path(__file__).resolve().parent / "fixtures/application.log"
    sample = source.read_text()
    with tempfile.TemporaryDirectory(prefix="dfir-native-profiles-") as directory:
        path = pathlib.Path(directory) / "profiles.dfir"
        session = command("POST", "/session", {"capabilities": {"alwaysMatch": {"tauri:options": {"application": binary}}}})
        root = "/session/" + session["sessionId"]

        def script(code):
            return command("POST", root + "/execute/sync", {"script": code, "args": []})

        def invoke(name, payload=None):
            return command("POST", root + "/execute/async", {
                "script": "const done=arguments[arguments.length-1];window.__TAURI_INTERNALS__.invoke(" + json.dumps(name) + "," + json.dumps(payload or {}) + ").then(done).catch(e=>done({error:String(e)}));", "args": [],
            })

        def action(text):
            wait_for(lambda: script("return [...document.querySelectorAll('button')].some(b=>b.textContent.trim()===" + json.dumps(text) + "&&!b.disabled)"))
            script("[...document.querySelectorAll('button')].find(b=>b.textContent.trim()===" + json.dumps(text) + ").click()")

        def fill(label, value):
            script("const e=document.querySelector('[aria-label=" + json.dumps(label) + "]');Object.getOwnPropertyDescriptor(e.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(e," + json.dumps(value) + ");e.dispatchEvent(new Event('input',{bubbles:true}));")

        def select(label, value):
            script("const e=document.querySelector('[aria-label=" + json.dumps(label) + "]');e.value=" + json.dumps(value) + ";e.dispatchEvent(new Event('change',{bubbles:true}));")

        def profiles():
            action("Parsers")
            wait_for(lambda: script("return !!document.querySelector('[aria-label=\"Parsing profile\"]')&&!document.querySelector('[aria-label=\"Parsing profile\"]').disabled"))

        def import_source(total):
            action("Import log file")
            location(source)
            wait_for(lambda: script("return document.querySelector('.table-footer')?.textContent.includes(" + json.dumps(f"1–{total} of {total} events") + ")"))

        def case_action(name):
            action("Case")
            script("document.querySelector('[data-case-action=" + json.dumps(name) + "]').click()")

        try:
            wait_for(lambda: script("return !!document.querySelector('.sample-button')"))
            profiles()
            fill("Profile sample", sample)
            action("Preview profile")
            wait_for(lambda: script("return document.querySelector('[aria-label=\"Profile preview results\"]')?.textContent.includes('4 records · 2 Parsed · 1 Partial · 1 Unparsed')"))
            assert invoke("case_summary")["sources"] == []
            pattern = script("return document.querySelector('[aria-label=\"Profile pattern\"]').value")
            fill("Profile pattern", "(")
            action("Preview profile")
            wait_for(lambda: script("return document.querySelector('.profile-diagnostic')?.textContent.includes('Invalid')"))
            assert invoke("parsing_profiles") == []
            fill("Profile pattern", pattern)
            action("Save as new profile")
            wait_for(lambda: len(invoke("parsing_profiles")) == 1)
            first = invoke("parsing_profiles")[0]
            action("Use revision 1 for next import")
            import_source(4)
            original = invoke("event_detail", {"id": 1})
            assert original["raw"] == sample.splitlines()[0]
            assert original["jsonFields"]["level"] == "INFO"
            assert invoke("case_summary")["sources"][0]["profile"] == first
            profiles()
            select("Parsing profile", "1")
            fill("Profile pattern", "(?P<message>.*)")
            action("Save new revision")
            wait_for(lambda: len(invoke("parsing_profiles")) == 2)
            second = invoke("parsing_profiles")[1]
            assert second["revision"] == 2 and first["sha256"] != second["sha256"]
            action("Use revision 2 for next import")
            import_source(8)
            assert invoke("event_detail", {"id": 1}) == original
            assert invoke("event_detail", {"id": 6})["parseStatus"] == "parsed"
            profiles()
            select("Parsing profile", "1")
            select("Profile revision", "1")
            assert script("return document.querySelector('[aria-label=\"Profile pattern\"]').value") == pattern
            action("Use revision 1 for next import")
            import_source(12)
            case_action("save")
            location(path)
            wait_for(lambda: invoke("case_summary").get("path") == str(path))
            case_action("new")
            wait_for(lambda: invoke("case_summary")["sources"] == [])
            assert invoke("parsing_profiles") == []
            case_action("open")
            location(path)
            wait_for(lambda: invoke("case_summary").get("path") == str(path))
            assert invoke("parsing_profiles") == [first, second]
            summaries = invoke("case_summary")["sources"]
            assert [s["profile"]["revision"] for s in summaries] == [1, 2, 1]
            assert invoke("event_detail", {"id": 1}) == original
            result = invoke("query_events", {"query": {"parser": "custom", "jsonFilter": {"key": "level", "value": "ERROR"}, "limit": 500}})
            assert result["total"] == 2
            profiles()
            select("Parsing profile", "1")
            select("Profile revision", "1")
            fill("Profile sample", sample)
            action("Preview profile")
            wait_for(lambda: script("return document.querySelectorAll('.profile-result').length===4"))
            screenshot = command("GET", root + "/screenshot")
            pathlib.Path("/tmp/dfir-delivery4-native.png").write_bytes(base64.b64decode(screenshot))
            select("Theme preference", "dark")
            wait_for(lambda: script("return document.documentElement.dataset.theme==='dark'"))
            command("POST", root + "/window/rect", {"width": 1000, "height": 720})
            wait_for(lambda: script("return window.innerWidth<=1000"))
            assert script("return document.documentElement.scrollWidth<=window.innerWidth")
            shot = command("GET", root + "/screenshot")
            pathlib.Path("/tmp/dfir-delivery4-native-dark.png").write_bytes(base64.b64decode(shot))
            # A profiles-only temporary case must not disappear through New case.
            case_action("new")
            wait_for(lambda: invoke("case_summary")["sources"] == [])
            profiles()
            action("Save as new profile")
            wait_for(lambda: len(invoke("parsing_profiles")) == 1)
            case_action("new")
            wait_for(lambda: script("return document.body.textContent.includes('Save this temporary case before switching so your parsing profiles are retained.')"))
            assert len(invoke("parsing_profiles")) == 1
            print("Native Delivery 4 passed: preview/invalid rules, immutable revisions, explicit historical/current imports through real dialogs, per-source definitions and unchanged evidence, save/new/reopen history, attribute query, and protection of unsaved profiles.")
        except Exception:
            try:
                shot = command("GET", root + "/screenshot")
                pathlib.Path("/tmp/dfir-delivery4-native-failure.png").write_bytes(base64.b64decode(shot))
            except Exception:
                pass
            raise
        finally:
            command("DELETE", root)


if __name__ == "__main__":
    main()
