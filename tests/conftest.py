import subprocess
import pytest
import time
import uuid
import re

@pytest.fixture(scope="session")
def docker_cmd():
    """Return the base docker command prefix, searching common paths for Mac parity."""
    paths = ["docker", "/usr/local/bin/docker", "/opt/homebrew/bin/docker"]
    for p in paths:
        try:
            res = subprocess.run([p, "ps"], capture_output=True)
            if res.returncode == 0:
                return [p]
        except Exception:
            continue
    return ["sudo", "docker"]

def run_docker(docker_cmd_prefix, args, capture=True):
    """Utility to run a docker command and return stdout, stderr, returncode."""
    cmd = docker_cmd_prefix + args
    res = subprocess.run(cmd, capture_output=capture, text=True)
    return res.returncode, res.stdout, res.stderr

@pytest.fixture
def run_in_kid_env(docker_cmd):
    """Fixture that returns a function to execute a command inside a throwaway kid container (unrestricted PATH)."""
    def _run(command, env=None):
        # Explicitly call /bin/zsh -c to avoid entrypoint confusion
        cmd = ["run", "--rm", "kid-env:latest", "/bin/zsh", "-c", command]
        return run_docker(docker_cmd, cmd)
    return _run

@pytest.fixture
def run_in_restricted_env(docker_cmd):
    """Fixture to execute commands with the fully restricted kid environment."""
    def _run(command):
        # Use ';' instead of '&&' to ensure command runs even if .zshrc has minor errors
        full_command = f"source /home/kid/.zshrc; {command}"
        cmd = ["run", "--rm", "-e", "TMUX=dummy", "kid-env:latest", "/bin/zsh", "-c", full_command]
        return run_docker(docker_cmd, cmd)
    return _run

class TmuxTester:
    def __init__(self, container_name, docker_cmd_prefix):
        self.container_name = container_name
        self.docker_cmd = docker_cmd_prefix

    def exec(self, command, env=None, login=True):
        """Execute a command in the container."""
        env_args = []
        if env:
            for k, v in env.items():
                env_args.extend(["-e", f"{k}={v}"])
        
        shell = ["/bin/zsh", "-l", "-c"] if login else ["/bin/sh", "-c"]
        cmd = ["exec", "-u", "kid"] + env_args + [self.container_name] + shell + [command]
        return run_docker(self.docker_cmd, cmd)

    def send_keys(self, keys):
        """Send keys via raw tmux call. Uses system default socket."""
        cmd = f"tmux send-keys -t kid_session '{keys}' C-m"
        # Using login=False for speed; signal wait ensures readiness
        rc, out, err = self.exec(cmd, login=False)
        return rc, out, err

    def capture_pane(self, target="kid_session"):
        """Capture pane content via raw tmux call."""
        cmd = f"tmux capture-pane -p -t {target}"
        rc, out, err = self.exec(cmd, login=False)
        return out if rc == 0 else ""

    def get_panes(self):
        """Get a list of all panes in the current session."""
        cmd = "tmux list-panes -F '#{pane_id},#{pane_title},#{pane_width},#{pane_height},#{pane_left},#{pane_top}'"
        rc, out, err = self.exec(cmd, login=False)
        
        panes = []
        if rc == 0:
            for line in out.strip().split("\n"):
                if not line: continue
                parts = line.split(",")
                if len(parts) >= 6:
                    panes.append({
                        "id": parts[0],
                        "title": parts[1],
                        "width": int(parts[2]),
                        "height": int(parts[3]),
                        "left": int(parts[4]),
                        "top": int(parts[5])
                    })
        panes.sort(key=lambda p: (p["top"], p["left"]))
        return panes

    def get_companion_pane(self):
        """Find the pane titled 'Companion' or the rightmost pane."""
        panes = self.get_panes()
        for p in panes:
            title = p.get("title", "").lower()
            if "companion" in title:
                return p
        if len(panes) <= 1:
            return None
        return max(panes, key=lambda p: p["left"])

    def wait_for_condition(self, condition_func, timeout=5.0, interval=0.1):
        """Poll a condition until it returns truthy or timeout."""
        start = time.time()
        while time.time() - start < timeout:
            res = condition_func()
            if res:
                return res
            time.sleep(interval)
        return None

    def wait_for_companion_pane(self, timeout=5.0):
        """Smart wait for companion pane to exist."""
        return self.wait_for_condition(lambda: self.get_companion_pane(), timeout=timeout)

    def wait_for_pane_text(self, pane_id, text, timeout=5.0):
        """Smart wait for specific text to appear in a pane."""
        def check():
            content = self.capture_pane(pane_id)
            return content if text in content or text.lower() in content.lower() else None
        return self.wait_for_condition(check, timeout=timeout)

@pytest.fixture
def tmux_session(docker_cmd):
    """Fixture that starts a detached container with Final Reality stability."""
    container_name = f"kid-test-{uuid.uuid4().hex[:8]}"
    session_name = "kid_session"
    signal_file = "/tmp/kid_ready"
    
    # 1. Start persistent container with --init for proper reaping
    run_docker(docker_cmd, [
        "run", "-d", "-t", "--name", container_name,
        "--init",
        "-u", "kid",
        "kid-env:latest", "sleep", "infinity"
    ])
    
    tester = TmuxTester(container_name, docker_cmd)
    
    # 2. Setup environment. Use system default socket.
    tester.exec(f"rm -f {signal_file}", login=False)
    # Start tmux session explicitly
    tester.exec(f"tmux new-session -d -s {session_name} -x 80 -y 24", login=False)
    # Ghost client attachment (kick PTY into gear)
    tester.exec(f"tmux attach-session -t {session_name} -d", login=False)
    
    # Wait for the SIGNAL (kid_ready)
    ready = False
    for i in range(30):
        rc, _, _ = tester.exec(f"test -f {signal_file}", login=False)
        if rc == 0:
            ready = True
            time.sleep(1.0) # Prompt settle
            break
        time.sleep(1)
    
    if not ready:
        _, out_ps, _ = tester.exec("ps aux", login=False)
        print(f"CRITICAL: Final Reality signal {signal_file} NOT found after 30s. PS AUX:\n{out_ps}")

    # Yield the tester back to tests
    yield tester
    
    # 3. Teardown & Validation Verification
    rc_log, err_log, _ = tester.exec("cat /tmp/kid_watch.err 2>/dev/null", login=False)
    _, out_log, _ = tester.exec("cat /tmp/kid_watch.log 2>/dev/null || echo 'No log'", login=False)
    _, pane_log, _ = tester.exec("cat /tmp/pane_debug.txt 2>/dev/null || echo 'No debug target'", login=False)
    _, out_ps, _ = tester.exec("ps aux", login=False)
    
    # Dump active panes just in case of failure for diagnostics
    panes = tester.get_panes()
    pane_dumps = []
    for p in panes:
        pane_dumps.append(f"--- PANE {p['id']} ---\n{tester.capture_pane(p['id'])}\n")
    all_panes = "\n".join(pane_dumps)

    print(f"\n--- DAEMON OUT ---\n{out_log}\n--- TARGET MSG ---\n{pane_log}\n--- PS AUX ---\n{out_ps}\n{all_panes}\n-------------------")
    
    # Ensure zero crashed panic traces emitted. Allow empty output to pass.
    assert err_log.strip() == "", f"CRITICAL ERR LOG TRIGGERED: {err_log}"

    run_docker(docker_cmd, ["rm", "-f", container_name])
