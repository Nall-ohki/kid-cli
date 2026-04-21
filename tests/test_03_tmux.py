import pytest
import time

def test_tmux_001_say_opens_companion(tmux_session):
    # Verify we start with 1 pane (or the daemon opens one)
    tmux_session.send_keys("say hello")
    
    comp_pane = tmux_session.wait_for_companion_pane()
    if comp_pane is None:
        panes = tmux_session.get_panes()
        if panes:
            out = tmux_session.capture_pane(panes[0]["id"])
            rc, ls_out, _ = tmux_session.exec("ls -la /tmp")
            rc, ps_out, _ = tmux_session.exec("ps aux")
            print(f"\n--- MAIN PANE LOGS ---\n{out}\n\n--- LS /TMP ---\n{ls_out}\n\n--- PS AUX ---\n{ps_out}\n-----------------------")
        assert comp_pane is not None, "Companion pane did not open"
    
    # Smart wait for the text to appear
    out = tmux_session.wait_for_pane_text(comp_pane["id"], "hello")
    captured = tmux_session.capture_pane(comp_pane['id'])
    
    # Strictly validate against shell parser breakdown and crash stacktraces
    if "Syntax error" in captured or "command not found" in captured or "sh: " in captured:
        pytest.fail(f"Companion pane executed with shell syntax errors:\n{captured}")
        
    assert out is not None, f"Expected 'hello' in companion pane, but found: {captured}"

def test_tmux_002_matrix_opens_bottom(tmux_session):
    panes = tmux_session.get_panes()
    # Initial state might have companion pane already
    initial_count = len(panes)
    
    tmux_session.send_keys("matrix")
    
    # Smart wait for any new pane to appear
    panes = tmux_session.wait_for_condition(lambda: tmux_session.get_panes() if len(tmux_session.get_panes()) > initial_count else None)
    if panes is None:
        panes = tmux_session.get_panes() # Get current state for error msg
    
    assert len(panes) > initial_count, "Bottom pane did not split"
    
    # matrix uses configured popup/pane settings in commands.toml
    # Default for matrix was split-window -v -p 30 in our implementation
    new_pane = panes[-1]
    assert new_pane["top"] > 0, "Matrix pane did not open on the bottom"
    
    # Smart wait for the text to appear
    out = tmux_session.wait_for_pane_text(new_pane["id"], "matrix")
    captured = tmux_session.capture_pane(new_pane["id"])
    
    # Strictly validate against shell parser breakdown
    if "Syntax error" in captured or "command not found" in captured or "sh: " in captured:
        pytest.fail(f"Bottom pane executed with shell syntax errors:\n{captured}")

def test_tmux_004_cd_valid_dir(tmux_session):
    # Test that cd triggers a companion message
    tmux_session.send_keys("cd apps")
    
    comp_pane = tmux_session.wait_for_companion_pane()
    assert comp_pane is not None, "Companion pane did not open on cd"
    
    out = tmux_session.wait_for_pane_text(comp_pane["id"], "Apps")
    assert out is not None

def test_tmux_005_ls_shows_companion(tmux_session):
    tmux_session.send_keys("ls")
    
    comp_pane = tmux_session.wait_for_companion_pane()
    assert comp_pane is not None

def test_exit_kills_tmux(tmux_session):
    # 'exit' in the new architecture kills the session
    tmux_session.send_keys("exit")
    
    # Poll for empty panes
    panes = tmux_session.wait_for_condition(lambda: [] if len(tmux_session.get_panes()) == 0 else None)
    if panes is None:
        panes = tmux_session.get_panes()
        
    assert len(panes) == 0, "Exit should have killed the tmux session completely"
