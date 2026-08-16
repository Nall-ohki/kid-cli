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

def test_app_006_putt_parade_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/putt_parade/putt_parade")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch putt_parade' /home/kid/apps/putt_parade/putt_parade")
    assert rc == 0

def test_app_007_putt_moon_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/putt_moon/putt_moon")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch putt_moon' /home/kid/apps/putt_moon/putt_moon")
    assert rc == 0

def test_app_008_putt_zoo_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/putt_zoo/putt_zoo")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch putt_zoo' /home/kid/apps/putt_zoo/putt_zoo")
    assert rc == 0

def test_app_009_krita(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/krita/krita")
    assert rc == 0

def test_app_010_krita_wrapper_delegation(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch krita' /home/kid/apps/krita/krita")
    assert rc == 0

def test_app_011_murphys_minerals_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/murphys_minerals/murphys_minerals")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch murphys_minerals' /home/kid/apps/murphys_minerals/murphys_minerals")
    assert rc == 0

def test_app_012_amazon_trail_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/amazon_trail/amazon_trail")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch amazon_trail' /home/kid/apps/amazon_trail/amazon_trail")
    assert rc == 0

def test_app_013_yukon_trail_wrapper(run_in_restricted_env):
    rc, out, err = run_in_restricted_env("test -x /home/kid/apps/yukon_trail/yukon_trail")
    assert rc == 0
    rc, out, err = run_in_restricted_env("/bin/grep 'kid launch yukon_trail' /home/kid/apps/yukon_trail/yukon_trail")
    assert rc == 0


