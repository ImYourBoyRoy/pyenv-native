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

def stdlib_module_names():
    """Return stdlib/builtin names, preferring sys.stdlib_module_names when available."""
    names = set(n.lower() for n in getattr(sys, 'stdlib_module_names', ()))
    names.update(n.lower() for n in sys.builtin_module_names)
    # Fallback coverage for older interpreters / incomplete platform lists
    names.update({
        'abc', 'argparse', 'ast', 'asyncio', 'base64', 'bisect', 'builtins', 'calendar', 'cmath',
        'cmd', 'code', 'codecs', 'collections', 'colorsys', 'compileall', 'concurrent', 'configparser',
        'contextlib', 'contextvars', 'copy', 'copyreg', 'crypt', 'csv', 'ctypes', 'curses', 'dataclasses',
        'datetime', 'dbm', 'decimal', 'difflib', 'dis', 'distutils', 'doctest', 'email', 'encodings',
        'ensurepip', 'enum', 'errno', 'faulthandler', 'filecmp', 'fileinput', 'fnmatch', 'fractions',
        'ftplib', 'functools', 'gc', 'getopt', 'getpass', 'gettext', 'glob', 'grp', 'gzip',
        'hashlib', 'hmac', 'html', 'http', 'imaplib', 'imghdr', 'importlib', 'inspect', 'io', 'ipaddress',
        'itertools', 'json', 'keyword', 'lib2to3', 'linecache', 'locale', 'logging', 'lzma', 'mailbox',
        'mailcap', 'marshal', 'math', 'mimetypes', 'mmap', 'modulefinder', 'multiprocessing', 'netrc',
        'nis', 'nntplib', 'numbers', 'operator', 'optparse', 'os', 'pathlib', 'pdb', 'pickle', 'pickletools',
        'pipes', 'pkgutil', 'platform', 'plistlib', 'poplib', 'posix', 'pprint', 'profile', 'pstats',
        'pty', 'pwd', 'py_compile', 'pyclbr', 'pydoc', 'queue', 'quopri', 'random', 're', 'readline',
        'reprlib', 'resource', 'rlcompleter', 'runpy', 'sched', 'secrets', 'select', 'selectors', 'shelve',
        'shlex', 'shutil', 'signal', 'site', 'smtpd', 'smtplib', 'sndhdr', 'socket', 'socketserver',
        'spwd', 'sqlite3', 'ssl', 'stat', 'statistics', 'string', 'stringprep', 'struct', 'subprocess',
        'sunau', 'symtable', 'sys', 'sysconfig', 'syslog', 'tabnanny', 'tarfile', 'telnetlib', 'tempfile',
        'termios', 'test', 'textwrap', 'threading', 'time', 'timeit', 'tkinter', 'token', 'tokenize',
        'tomllib', 'trace', 'traceback', 'tracemalloc', 'tty', 'types', 'typing', 'unicodedata', 'unittest',
        'urllib', 'uu', 'uuid', 'warnings', 'wave', 'weakref', 'webbrowser', 'wsgiref', 'xdrlib', 'xml',
        'xmlrpc', 'zipfile', 'zipimport', 'zlib', 'zoneinfo',
    })
    return names


def is_non_pip_import(name):
    """Skip stdlib, dunder modules, and private/C-extension stubs that cannot be pip-installed."""
    if not name:
        return True
    lowered = name.lower()
    if lowered in stdlib_module_names():
        return True
    if lowered.startswith('__') and lowered.endswith('__'):
        return True
    if lowered in {'__main__', '__pypy__', '__mp_main__'}:
        return True
    # Private CPython internals (_ssl, _io, _pickle, …) are never pip packages
    if lowered.startswith('_'):
        return True
    return False


def scan_workspace_imports(dir_path):
    import os
    import ast
    import importlib.util

    imported = set()
    ignored_dirs = {
        '.git', '.venv', 'venv', '__pycache__', 'build', 'dist', 'target',
        '.gemini', 'node_modules', '.tox', '.mypy_cache', '.pytest_cache', '.ruff_cache',
    }
    for root, dirs, files in os.walk(dir_path):
        dirs[:] = [d for d in dirs if d not in ignored_dirs]
        if any(part in ignored_dirs for part in root.split(os.sep)):
            continue
        for file in files:
            if not file.endswith('.py'):
                continue
            file_path = os.path.join(root, file)
            try:
                with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                    tree = ast.parse(f.read(), filename=file_path)
                for node in ast.walk(tree):
                    if isinstance(node, ast.Import):
                        for alias in node.names:
                            imported.add(alias.name.split('.')[0])
                    elif isinstance(node, ast.ImportFrom):
                        if getattr(node, 'level', 0):
                            continue  # relative import
                        if node.module:
                            imported.add(node.module.split('.')[0])
            except Exception:
                pass

    third_party = set()
    for name in imported:
        if is_non_pip_import(name):
            continue
        try:
            spec = importlib.util.find_spec(name)
            if spec is None:
                third_party.add(name)
            else:
                origin = getattr(spec, 'origin', '') or ''
                if 'site-packages' in origin or 'dist-packages' in origin:
                    third_party.add(name)
        except Exception:
            if not is_non_pip_import(name):
                third_party.add(name)

    installed = get_installed()
    detected = sorted(list(third_party))

    missing = []
    installed_imports = []

    for name in detected:
        name_lower = name.lower()
        if name_lower in installed:
            installed_imports.append({"name": name, "version": installed[name_lower]})
        else:
            missing.append(name)

    return {
        "detected_imports": detected,
        "missing_imports": missing,
        "installed_imports": installed_imports,
        "skipped_non_pip": True,
    }

def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "Missing path or URL argument"}))
        return
    
    first_arg = sys.argv[1]
    
    if first_arg == '--scan':
        if len(sys.argv) < 3:
            print(json.dumps({"error": "Missing workspace directory path for scan"}))
            return
        workspace_dir = sys.argv[2]
        try:
            result = scan_workspace_imports(workspace_dir)
            print(json.dumps(result))
        except Exception as e:
            print(json.dumps({"error": str(e)}))
        return
        
    try:
        reqs = get_requirements(first_arg)
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
