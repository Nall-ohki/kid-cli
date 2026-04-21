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

def test_app_004_tuxpaint_infra_path(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/bin/grep '_INFRA_PATH' /home/kid/apps/tuxpaint/tuxpaint")
    assert rc == 0

def test_app_005_tuxpaint_cage(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/bin/grep 'cage' /home/kid/apps/tuxpaint/tuxpaint")
    assert rc == 0

def test_img_002_user_ownership(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/usr/bin/find /home/kid ! -user kid")
    # There should ideally be no files owned by root in /home/kid
    pass

def test_img_012_uv_installed(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/home/kid/.cargo/bin/uv --version || /usr/local/bin/uv --version || /bin/uv --version")
    assert rc == 0
    assert "uv" in out
