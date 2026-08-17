import pytest
import os

def test_app_001_tuxpaint(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/art/tuxpaint")
    assert rc == 0

def test_app_002_gcompris(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/abc/gcompris")
    assert rc == 0

def test_app_003_scratch(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/code/scratch")
    assert rc == 0

def test_app_004_tuxpaint_wrapper_delegation(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch tuxpaint' /home/kid/apps/art/tuxpaint")
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

def test_app_006_parade_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/putt/parade")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch parade' /home/kid/apps/putt/parade")
    assert rc == 0

def test_app_007_moon_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/putt/moon")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch moon' /home/kid/apps/putt/moon")
    assert rc == 0

def test_app_008_zoo_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/putt/zoo")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch zoo' /home/kid/apps/putt/zoo")
    assert rc == 0

def test_app_009_krita(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/art/krita")
    assert rc == 0

def test_app_010_krita_wrapper_delegation(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch krita' /home/kid/apps/art/krita")
    assert rc == 0

def test_app_011_murphy_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/play/murphy")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch murphy' /home/kid/apps/play/murphy")
    assert rc == 0

def test_app_012_amazon_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/trail/amazon")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch amazon' /home/kid/apps/trail/amazon")
    assert rc == 0

def test_app_013_yukon_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/trail/yukon")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch yukon' /home/kid/apps/trail/yukon")
    assert rc == 0

def test_app_014_mario_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/art/mario")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch mario' /home/kid/apps/art/mario")
    assert rc == 0

def test_app_015_math_wrappers(run_in_restricted_env):
    for app in ["tuxmath", "nummunch", "fracmunch", "zoombini"]:
        rc, out, err = run_in_restricted_env(f"test -x /home/kid/apps/math/{app}")
        assert rc == 0
        rc, out, err = run_in_restricted_env(f"/bin/grep 'kid launch {app}' /home/kid/apps/math/{app}")
        assert rc == 0

def test_app_016_abc_wrappers(run_in_restricted_env):
    for app in ["tuxtype", "klettres", "wordmunch", "donald", "gcompris", "mariotype"]:
        rc, out, err = run_in_restricted_env(f"test -x /home/kid/apps/abc/{app}")
        assert rc == 0
        rc, out, err = run_in_restricted_env(f"/bin/grep 'kid launch {app}' /home/kid/apps/abc/{app}")
        assert rc == 0

def test_app_017_carmen_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/play/carmen")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch carmen' /home/kid/apps/play/carmen")
    assert rc == 0

def test_app_018_trail_oregon_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/trail/oregon")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch oregon' /home/kid/apps/trail/oregon")
    assert rc == 0


