#!/usr/bin/env python3
"""CI's `tests` job and flake.nix's `checks`, read strictly: the one list of what CI runs.

`tools/check_all.sh` runs what this reads, and `tools/check_doc_links.py`
holds CLAUDE.md's table of checks to it, so the list is written once, in
`ci.yml`, and everything else reads it.

Anything whose meaning a plain line reader could get wrong is refused rather
than guessed: a step key outside name/id/if/run/uses/with (`shell:`, `env:`,
`working-directory:` and `continue-on-error:` all change what a `run:` does), a
block scalar other than `|`, a quoted `run:`, a workflow or job key that changes
how steps run (`env:`, `defaults:`, `container:`), and a job that is neither
`tests` nor one the caller excludes. The dev shell has no yaml module.

    tools/ci_steps.py STEPS FLAKE [EXCLUDED_JOB...]

writes `kind \\0 key \\0 if \\0 body \\0` per step of `tests` to STEPS (kind is
`run` or `uses`, key the step's name or else its body), and to FLAKE the
command `check_all.sh --fast` runs for each of flake.nix's `checks`, one a line.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CI = ROOT / ".github/workflows/ci.yml"
FLAKE = ROOT / "flake.nix"

TOP = {"name", "on", "concurrency", "jobs", "permissions"}
JOB = {"runs-on", "permissions", "steps"}
STEP = {"name", "id", "if", "run", "uses", "with"}

# Each builder flake.nix's checks may use, the command that stands in for it
# outside nix, and the arguments whose effect that command already states.
STANDINS = {
    "self.packages.${system}.default;": ("cargo build --release --workspace", set()),
    "craneLib.cargoFmt": ("cargo fmt --check", set()),
    "craneLib.cargoClippy": (
        "cargo clippy {cargoClippyExtraArgs}",
        {"inherit cargoArtifacts", "cargoClippyExtraArgs"},
    ),
    "craneLib.cargoNextest": (
        "cargo nextest run",
        {"inherit cargoArtifacts", "partitions", "partitionType"},
    ),
    "craneLib.cargoDoc": (
        "env RUSTDOCFLAGS={RUSTDOCFLAGS!r} cargo doc {cargoDocExtraArgs}",
        {"inherit cargoArtifacts", "RUSTDOCFLAGS", "cargoDocExtraArgs"},
    ),
    "craneLib.cargoDocTest": ("cargo test --doc", {"inherit cargoArtifacts"}),
}


def read_steps(excluded_jobs=(), ci=CI):
    """(steps, errors): each step of the `tests` job as a dict of its keys."""
    errors = []
    lines = ci.read_text().split("\n")
    indent = lambda l: len(l) - len(l.lstrip(" "))
    steps, jobs, job, cur, where, section, i = [], [], None, None, None, None, 0
    while i < len(lines):
        line, n = lines[i], i + 1
        i += 1
        text = line.strip()
        if not text or text.startswith("#"):
            continue
        d = indent(line)
        key = re.match(r"^(?:-\s+)?(?P<k>[A-Za-z_-]+):(?:\s|$)", text)
        if d == 0:
            where = key and key["k"]
            if where not in TOP:
                errors.append(f"ci.yml:{n}: workflow key {text!r} is not one this reads")
            continue
        if where != "jobs":
            continue
        if d == 2:
            job = key and key["k"]
            jobs.append(job)
            if job != "tests" and job not in excluded_jobs:
                errors.append(f"ci.yml:{n}: job {job!r} is neither `tests` nor excluded in check_all.sh")
            continue
        if job != "tests":
            continue
        if d == 4:
            section = key and key["k"]
            if section not in JOB:
                errors.append(f"ci.yml:{n}: tests job key {text!r} is not one this reads")
            continue
        if section != "steps":
            continue
        if d == 6 and text.startswith("- "):
            cur = {}
            steps.append(cur)
            d, text = 8, text[2:].strip()
            key = re.match(r"^(?P<k>[A-Za-z_-]+):(?:\s|$)", text)
        if d == 8:
            k = key and key["k"]
            if cur is None or k not in STEP:
                errors.append(f"ci.yml:{n}: step key {text!r} is not one of {sorted(STEP)}")
                continue
            val = text.split(":", 1)[1].strip()
            if k == "run":
                if val.startswith(("|", ">")) and val != "|":
                    errors.append(f"ci.yml:{n}: block scalar {val!r}; only `run: |` is read")
                    continue
                if val.startswith(("'", '"')):
                    errors.append(f"ci.yml:{n}: quoted run: is not read; write it plain or as `|`")
                    continue
                if val == "|":
                    block = []
                    while i < len(lines) and (indent(lines[i]) >= 10 or not lines[i].strip()):
                        block.append(lines[i][10:])
                        i += 1
                    val = "\n".join(block).strip() + "\n"
            cur[k] = val
            continue
        if d >= 10 and cur is not None and "with" in cur:
            continue
        errors.append(f"ci.yml:{n}: line not read: {text!r}")

    for e in excluded_jobs:
        if e not in jobs:
            errors.append(f"excluded job {e!r} is not in ci.yml")
    if "tests" not in jobs:
        errors.append("ci.yml has no tests job")
    for s in steps:
        if ("run" in s) == ("uses" in s):
            errors.append(f"ci.yml: a step has {'both' if 'run' in s else 'neither'} run: and uses: {s!r}")
    return steps, errors


def read_flake(flake=FLAKE):
    """(standins, errors): per flake.nix check, the cargo command it runs."""
    errors, standins = [], []
    src = flake.read_text()
    m = re.search(r"\n      checks = eachSystem.*?\n        \{\n(.*?)\n        \}\);", src, re.S)
    if not m:
        return [], ["flake.nix: no `checks = eachSystem` block"]
    entries = re.split(r"\n(?=          [a-z]\w* = )", "\n" + m.group(1))
    for entry in filter(str.strip, entries):
        name, body = entry.strip().split(" = ", 1)
        inner = set(re.findall(r"^\s+(\w+)\s*=", body, re.M))
        inner |= {"inherit " + w for w in re.findall(r"inherit (\w+);", body)}
        # The builder is the body's first word, matched whole: `cargoDoc` is a
        # prefix of `cargoDocTest`.
        first = body.split(None, 1)[0]
        head = first if first in STANDINS else None
        if head is None:
            errors.append(f"flake.nix check {name!r}: {body.splitlines()[0]!r} has no cargo stand-in here")
            continue
        cmd, allowed = STANDINS[head]
        extra = inner - allowed
        if extra:
            errors.append(f"flake.nix check {name!r}: arguments {sorted(extra)} have no stand-in here")
            continue
        args = dict(re.findall(r'(\w+) = "([^"]*)";', body))
        try:
            standins.append(cmd.format(**args))
        except KeyError as missing:
            errors.append(f"flake.nix check {name!r}: its stand-in needs {missing}")
    return standins, errors


def read_jobs(ci=CI):
    """The workflow's top-level keys and each job's keys, as indented text:
    `{"": {key: text}, job: {key: text}}`, a key's text being its value and
    every line nested under it, stripped and joined by newlines."""
    out = {"": {}}
    top = job = key = None
    for line in ci.read_text().split("\n"):
        text = line.strip()
        if not text or text.startswith("#"):
            continue
        d = len(line) - len(line.lstrip(" "))
        m = re.match(r"^([A-Za-z_-]+):\s*(.*)$", text)
        if d == 0 and m:
            top, job, key = m.group(1), None, None
            out[""][top] = m.group(2)
        elif top == "jobs" and d == 2 and m:
            job, key = m.group(1), None
            out[job] = {}
        elif top == "jobs" and d == 4 and m and job:
            key = m.group(1)
            out[job][key] = m.group(2)
        elif top == "jobs" and job and key:
            out[job][key] += "\n" + text
        elif top != "jobs":
            out[""][top] += "\n" + text
    return out


def policy(ci=CI):
    """What the workflow may do, beyond which steps it runs. Errors, if any.

    - The workflow grants nothing at the top level; the `tests` job, which
      runs project code, reads the repository and nothing else.
    - Only `deploy` holds a write permission, and it runs on `main` alone.
    - Runs are grouped per ref (`${{ github.ref }}`), cancelling a running one
      on pull requests only; the deploy has a group of its own that never
      cancels a running deploy."""
    jobs = read_jobs(ci)
    top = jobs.pop("")
    errors = []
    if "permissions" in top:
        errors.append("ci.yml: permissions granted at the top level; grant them per job")
    conc = top.get("concurrency", "")
    if "${{ github.ref }}" not in conc:
        errors.append("ci.yml: the workflow's concurrency group is not per ref")
    if "cancel-in-progress: ${{ github.event_name == 'pull_request' }}" not in conc:
        errors.append("ci.yml: cancel-in-progress is not on pull requests alone")
    for name, job in jobs.items():
        perms = job.get("permissions", "")
        grants = dict(re.findall(r"^([a-z-]+):\s*(\S+)$", perms, re.M))
        writes = sorted(k for k, v in grants.items() if v == "write")
        if name == "tests" and grants != {"contents": "read"}:
            errors.append(f"ci.yml: job tests holds {grants}; it may read contents and nothing else")
        if name != "deploy" and writes:
            errors.append(f"ci.yml: job {name} writes {writes}; only deploy may write")
        if name == "deploy":
            if job.get("if", "").strip() != "github.ref == 'refs/heads/main'":
                errors.append("ci.yml: deploy does not run on main alone")
            c = job.get("concurrency", "")
            if "group: pages" not in c or "cancel-in-progress: false" not in c:
                errors.append("ci.yml: deploy is not serialised on its own group without cancelling")
        if "permissions" not in job:
            errors.append(f"ci.yml: job {name} states no permissions")
    return errors


def key(step):
    """What a step is known by: its name, or else its command, or its action
    without the version, so a version bump is not a new step."""
    if "name" in step:
        return step["name"]
    if "run" in step:
        return step["run"].strip()
    return step["uses"].split("@", 1)[0]


def main():
    steps_out, flake_out, *excluded = sys.argv[1:]
    steps, errors = read_steps(excluded)
    standins, more = read_flake()
    errors += more + policy()
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    with open(steps_out, "w") as f:
        for s in steps:
            kind = "run" if "run" in s else "uses"
            f.write(f"{kind}\0{key(s)}\0{s.get('if', '')}\0{s[kind]}\0")
    with open(flake_out, "w") as f:
        f.write("".join(c + "\n" for c in standins))
    return 0


if __name__ == "__main__":
    sys.exit(main())
