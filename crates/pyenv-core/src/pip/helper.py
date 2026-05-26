# ./crates/pyenv-core/src/pip/helper.py
"""Embedded helper utility for prechecking requirements.txt dependency safety.

Acts as a self-contained environment validator that resolves installed metadata,
performs HSL-aligned compatibility comparisons, automatically translates GitHub URLs,
and flags conflicts before any installation is written to disk.
"""

import sys
import json
import re
import urllib.request

# Modern python metadata or pkg_resources fallback
try:
    from importlib.metadata import distributions
    def get_installed():
        return {d.metadata['Name'].lower(): d.version for d in distributions()}
except ImportError:
    try:
        import pkg_resources
        def get_installed():
            return {p.project_name.lower(): p.version for p in pkg_resources.working_set}
    except Exception:
        def get_installed():
            return {}

# Fallback requirement parser for version comparison
COMP_RE = re.compile(r'^([a-zA-Z0-9_\-\[\]]+)\s*(>=|<=|==|!=|>|<|~=)\s*([a-zA-Z0-9_\-\.]+)')

def parse_requirement(line):
    line = line.strip()
    if not line or line.startswith(('#', '-', '_')):
        return None
    # Strip inline comments
    if ' #' in line:
        line = line.split(' #')[0].strip()
    m = COMP_RE.match(line)
    if m:
        name = m.group(1).split('[')[0].lower() # Strip extras
        op = m.group(2)
        ver = m.group(3)
        return {"name": name, "op": op, "ver": ver, "original": line}
    # Check for name-only requirement
    name_only = re.match(r'^([a-zA-Z0-9_\-\[\]]+)$', line)
    if name_only:
        name = name_only.group(1).split('[')[0].lower()
        return {"name": name, "op": "", "ver": "", "original": line}
    return None

def compare_versions(v1, op, v2):
    # Parse dotted version into tuple of ints/strings
    def to_tuple(v):
        return tuple(int(x) if x.isdigit() else x for x in re.split(r'[\.\-\+]', v))
    try:
        t1, t2 = to_tuple(v1), to_tuple(v2)
        if op == '==': return t1 == t2
        if op == '!=': return t1 != t2
        if op == '>=': return t1 >= t2
        if op == '<=': return t1 <= t2
        if op == '>': return t1 > t2
        if op == '<': return t1 < t2
        if op == '~=':
            if len(t1) < len(t2):
                return False
            return t1 >= t2 and t1[:len(t2)-1] == t2[:len(t2)-1]
    except Exception:
        pass
    return True

def get_requirements(path_or_url):
    lines = []
    if path_or_url.startswith(('http://', 'https://')):
        url = path_or_url
        if 'github.com' in url and '/blob/' in url:
            url = url.replace('github.com', 'raw.githubusercontent.com').replace('/blob/', '/')
        req = urllib.request.Request(url, headers={'User-Agent': 'pyenv-native'})
        with urllib.request.urlopen(req) as response:
            lines = response.read().decode('utf-8').splitlines()
    else:
        with open(path_or_url, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    
    parsed = []
    for line in lines:
        p = parse_requirement(line)
        if p:
            parsed.append(p)
    return parsed

def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "Missing path or URL argument"}))
        return
    
    path_or_url = sys.argv[1]
    try:
        reqs = get_requirements(path_or_url)
        installed = get_installed()
        
        resolved = []
        conflicts = []
        is_safe = True
        
        for r in reqs:
            name = r["name"]
            op = r["op"]
            ver = r["ver"]
            orig = r["original"]
            
            inst_ver = installed.get(name)
            if inst_ver:
                resolved.append({"name": name, "version": inst_ver})
                if op:
                    if not compare_versions(inst_ver, op, ver):
                        is_safe = False
                        conflicts.append({
                            "package": name,
                            "requirement": orig,
                            "installed": inst_ver,
                            "message": f"Installed version {inst_ver} violates requirement {orig}"
                        })
            else:
                # We don't mark not installed as unsafe, since the installation itself is safe (will install it).
                # But we can list it as a potential item or just note it.
                resolved.append({"name": name, "version": "not installed"})
        
        print(json.dumps({
            "is_safe": is_safe,
            "resolved_packages": resolved,
            "potential_conflicts": conflicts
        }))
    except Exception as e:
        print(json.dumps({"error": str(e)}))

if __name__ == '__main__':
    main()
