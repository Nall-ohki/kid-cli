import pytest
import time

def test_ls_001_home(tmux_session):
    tmux_session.send_keys("ls")
    time.sleep(1)
    out = tmux_session.capture_pane("%0")
    assert "apps" in out or "creations" in out

def test_ls_003_args(tmux_session):
    tmux_session.send_keys("ls ~/creations")
    time.sleep(1)
    out = tmux_session.capture_pane("%0")
    assert "pictures" in out or "programs" in out or "games" in out

def test_ls_005_nonexistent(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("ls /nonexistent")
    assert "No such file" in out or "No such file" in err or "cannot access" in out or "cannot access" in err
