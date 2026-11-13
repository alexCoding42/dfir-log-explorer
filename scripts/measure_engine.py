"""Record one Unix engine workload; invoke once per fresh benchmark process.

Usage: python scripts/measure_engine.py BINARY EVENT_COUNT OUTPUT_JSON
The child process RSS includes source generation and saving, as well as queries.
"""

import json
import pathlib
import platform
import resource
import subprocess
import sys
import time


def main():
    binary, count, output = sys.argv[1:]
    started = time.monotonic()
    result = subprocess.run(
        [binary, count], text=True, capture_output=True, check=True
    )
    wall_seconds = time.monotonic() - started
    peak_rss_kib = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    report = json.loads(result.stdout)
    report.update(
        process_wall_seconds=wall_seconds,
        process_peak_rss_kib=peak_rss_kib,
        platform=platform.platform(),
        cpu_quota=pathlib.Path("/sys/fs/cgroup/cpu.max").read_text().strip(),
        memory_limit=pathlib.Path("/sys/fs/cgroup/memory.max").read_text().strip(),
    )
    pathlib.Path(output).write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
