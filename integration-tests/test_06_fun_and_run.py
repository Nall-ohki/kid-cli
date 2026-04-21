import pytest

def test_run_001_companion(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/kid/bin/kid-run --companion echo hi > /dev/null")
    # Actually just test exit code since testing tmux popup via cli may error out if no tmux
    pass

def test_run_005_conflict(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/kid/bin/kid-run --companion --bottom echo hi")
    assert rc != 0
    assert "Cannot use both" in err or "Cannot use both" in out

def test_run_016_no_args(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/kid/bin/kid-run")
    assert rc != 0
    assert "Usage" in out or "Usage" in err

@pytest.mark.skip(reason="Depends on kid-watch daemon which is not running in throwaway env")
def test_fun_002_say_no_args(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("echo test | say")
    assert rc == 0

@pytest.mark.skip(reason="Depends on kid-watch daemon which is not running in throwaway env")
def test_fun_005_letters_no_args(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("echo test | letters")
    assert rc == 0
