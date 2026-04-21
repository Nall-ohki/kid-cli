import pytest
import time

@pytest.mark.xfail(reason="tmux display-popup requires an attached client, which is not available in headless tests")
def test_hlp_001_basic(tmux_session):
    tmux_session.send_keys("help")
    # Wait for the popup to open and show content
    out = tmux_session.wait_for_pane_text(None, "Sections", timeout=10.0)
    assert out is not None
    assert "Basic" in out
    assert "Fun" in out
    
    # Close it with 'q'
    tmux_session.send_keys("q")
    time.sleep(1)
    # verify it's gone (or at least we can type again)
    tmux_session.send_keys("echo ready")
    assert tmux_session.wait_for_pane_text(None, "ready") is not None

def test_exit_003_ctrl_d(tmux_session):
    tmux_session.send_keys("C-d")
    time.sleep(1)
    panes = tmux_session.get_panes()
    assert len(panes) > 0 # Should not close

def test_clr_001_clear(tmux_session):
    tmux_session.send_keys("clear")
    time.sleep(1)
    # verify it doesn't fail
    panes = tmux_session.get_panes()
    assert len(panes) > 0
