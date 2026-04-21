import pytest
import time

def test_cd_001_apps(tmux_session):
    tmux_session.send_keys("cd apps")
    comp_pane = tmux_session.wait_for_companion_pane()
    assert comp_pane is not None, "CD should trigger companion message"

def test_cd_006_no_args(tmux_session):
    tmux_session.send_keys("cd")
    # CD with no args returns target Ok(()) in validate, then shell does builtin cd
    # We verify it doesn't crash the session
    panes = tmux_session.get_panes()
    assert len(panes) >= 1

def test_cd_007_nonexistent(tmux_session):
    tmux_session.send_keys("cd nonexistent")
    # The kid msg should show Error sigil in the main pane
    out = tmux_session.wait_for_pane_text("kid_session", "Directory does not exist")
    assert out is not None

def test_msg_001_kid_msg(run_in_restricted_env):
    # Old test used /kid/bin/kid-error. New test uses /kid/bin/kid msg error
    rc, out, err = run_in_restricted_env("/kid/bin/kid msg error 'test_error_message'")
    assert "test_error_message" in out or "test_error_message" in err
