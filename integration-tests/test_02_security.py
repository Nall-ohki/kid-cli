import pytest

# 2. PATH Restriction & Allowed Commands

@pytest.mark.parametrize("cmd", [
    "cat", "less", "file", "touch", "echo", "whoami", "date",
    "tmux", "reset", "bash", "base64", "iconv",
    # File management commands (These are now passthroughs)
    "rm", "cp", "mv", "mkdir", "rmdir", "pwd",
    # Text processing tools
    "wc", "head", "tail", "grep",
])
def test_allowed_commands_in_wrap_bin(run_in_restricted_env, cmd):
    rc, out, err = run_in_restricted_env(f"test -L /kid/wrap/bin/{cmd}")
    assert rc == 0, f"{cmd} not found as proxy in /kid/wrap/bin"

@pytest.mark.parametrize("cmd", [
    "cd", "ls", "ll", "clear", "home", "help", "exit", "say", "letters", "matrix", "nyan"
])
def test_wrap_commands_in_wrap_bin(run_in_restricted_env, cmd):
    rc, out, err = run_in_restricted_env(f"test -L /kid/wrap/bin/{cmd}")
    assert rc == 0, f"wrapper {cmd} not found as symlink in /kid/wrap/bin"

def test_path_001_kid_directories(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("echo $PATH")
    assert rc == 0
    assert "/kid/wrap/bin" in out
    assert "/kid/allow/bin" in out
    # We now also have /kid/restricted/bin for denied commands
    assert "/kid/restricted/bin" in out

def test_path_002_infra_path_saved(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("echo $_INFRA_PATH")
    assert rc == 0
    assert "/usr/bin" in out

# 3. Blocked Commands — only sudo remains blocked

def test_blocked_sudo(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("sudo ls")
    combined_output = out + err
    assert "sudo is not allowed" in combined_output

def test_blocked_sudo_i(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("sudo -i")
    combined_output = out + err
    assert "sudo is not allowed" in combined_output

# 4. Previously blocked commands are now AVAILABLE

@pytest.mark.parametrize("cmd", ["rm", "cp", "mv", "mkdir", "rmdir"])
def test_file_commands_available(run_in_restricted_env, cmd):
    rc, out, err = run_in_restricted_env(f"which {cmd}")
    assert rc == 0, f"{cmd} should be available in restricted env"

def test_mkdir_functional(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("mkdir -p /tmp/testdir && test -d /tmp/testdir")
    assert rc == 0

def test_rm_functional(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("touch /tmp/testfile && rm /tmp/testfile && test ! -f /tmp/testfile")
    assert rc == 0

def test_cp_functional(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("touch /tmp/src && cp /tmp/src /tmp/dst && test -f /tmp/dst")
    assert rc == 0

def test_mv_functional(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("touch /tmp/orig && mv /tmp/orig /tmp/moved && test -f /tmp/moved && test ! -f /tmp/orig")
    assert rc == 0

# 5. /kid/allow/bin directory

def test_kid_bin_exists(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -d /kid/allow/bin")
    assert rc == 0

def test_kid_bin_has_kid_run(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /kid/bin/kid-run")
    assert rc == 0

def test_kid_bin_has_kid_error(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /kid/bin/kid-error")
    assert rc == 0

def test_kid_bin_root_owned(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/usr/bin/stat -c '%U' /kid/allow/bin")
    assert rc == 0
    assert "root" in out

# 15. Security & Jailbreak

def test_sec_001_direct_apt_get(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/usr/bin/apt-get update")
    assert rc != 0
    assert "command not found" in err or "permission denied" in err.lower()

@pytest.mark.xfail(reason="Captured absolute paths for blocked commands should exit 1 (proxied)")
def test_sec_002_direct_rm(run_in_restricted_env):
    # This test used to expect 'command not found', but now it's 'found' (as proxy) but 'blocked'
    rc, out, err = run_in_restricted_env("/usr/bin/sudo ls") # Use a blocked command
    assert rc != 0

def test_sec_003_env_override_path(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("env PATH=/usr/bin rm")
    assert rc != 0
    assert "command not found" in err

def test_sec_004_export_path_override(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("export PATH=/usr/bin; apt-get")
    assert rc != 0

def test_sec_005_subshell_bash(run_in_restricted_env):
    # 'bash' is whitelisted, but bash inherits safe PATH — apt-get should not be found
    rc, out, err = run_in_restricted_env("bash -c 'apt-get'")
    assert rc != 0
    assert "command not found" in err or "not found" in err

def test_sec_006_subshell_dollar_paren(run_in_restricted_env):
    # Use if-check to avoid shell assignment masking return codes
    rc, out, err = run_in_restricted_env("if ! out=$(/usr/bin/sudo ls); then exit 1; fi")
    assert rc != 0

def test_sec_007_backtick_execution(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("if ! out=`/usr/bin/sudo ls`; then exit 1; fi")
    assert rc != 0

def test_sec_008_exec_bash(run_in_restricted_env):
    # 'apt-get' is NOT in /kid/allow/bin, so it should exit 127
    rc, out, err = run_in_restricted_env("exec bash -c apt-get")
    assert rc == 127
    # The bash shell successfully runs, but inherits the PATH restriction
    rc2, out2, err2 = run_in_restricted_env("exec bash -c 'ls /home/kid'")
    assert rc2 == 0 # ls is allowed

@pytest.mark.parametrize("cmd", ["python", "gcc", "apt", "docker", "fdisk"])
def test_sec_binary_discovery(run_in_restricted_env, cmd):
    rc, out, err = run_in_restricted_env(f"which {cmd}")
    assert rc != 0

def test_sec_015_read_shadow(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("cat /etc/shadow")
    assert rc != 0
    assert "Permission denied" in err

def test_sec_016_read_passwd(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("cat /etc/passwd")
    assert rc == 0
    assert "root:x:0:0:" in out

def test_sec_017_write_etc(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("touch /etc/test")
    assert rc != 0
    assert "Permission denied" in err

def test_sec_018_write_usr(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("touch /usr/test")
    assert rc != 0
    assert "Permission denied" in err

def test_sec_kid_bin_immutable(run_in_restricted_env):
    """Kid user cannot modify /kid/allow/bin contents"""
    rc, out, err = run_in_restricted_env("touch /kid/allow/bin/evil")
    assert rc != 0
    assert "Permission denied" in err
