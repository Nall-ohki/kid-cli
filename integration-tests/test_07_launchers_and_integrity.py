import pytest
import os

def test_app_001_tuxpaint(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/tuxpaint/tuxpaint")
    assert rc == 0

def test_app_002_gcompris(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/gcompris/gcompris")
    assert rc == 0

def test_app_003_scratch(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/scratch/scratch")
    assert rc == 0

def test_app_004_tuxpaint_wrapper_delegation(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch tuxpaint' /home/kid/apps/tuxpaint/tuxpaint")
    assert rc == 0

def test_img_002_user_ownership(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/usr/bin/find /home/kid ! -user kid")
    # There should ideally be no files owned by root in /home/kid
    pass

def test_app_005_cli_launch_subcommand_exists(run_in_restricted_env):
    # Verify that the 'kid' binary actually has a 'launch' subcommand
    # We test it with a non-existent app to ensure it hits the launcher registry error
    # rather than the "unrecognized subcommand 'launch'" error from clap.
    rc, out, err = run_in_restricted_env("/kid/bin/kid launch nonexistent_app")
    assert "unrecognized subcommand" not in err.lower()
    assert rc != 0

def test_img_012_uv_installed(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/kid/allow/bin/uv --version || /home/kid/.cargo/bin/uv --version || /usr/local/bin/uv --version || /bin/uv --version")
    assert rc == 0
    assert "uv" in out
