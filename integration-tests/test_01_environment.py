import pytest

def test_env_001_kid_user_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("id kid")
    assert rc == 0
    assert "uid=" in out
    assert "gid=" in out

def test_env_002_kid_user_shell(run_in_kid_env):
    rc, out, err = run_in_kid_env("getent passwd kid | cut -d: -f7")
    assert rc == 0
    assert "/bin/zsh" in out

def test_env_003_kid_home_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -d /home/kid")
    assert rc == 0

def test_env_004_kid_in_video_group(run_in_kid_env):
    rc, out, err = run_in_kid_env("id -nG kid | grep -w video")
    assert rc == 0

def test_env_005_kid_in_render_group(run_in_kid_env):
    rc, out, err = run_in_kid_env("id -nG kid | grep -w render")
    assert rc == 0

def test_env_006_kid_in_input_group(run_in_kid_env):
    rc, out, err = run_in_kid_env("id -nG kid | grep -w input")
    assert rc == 0

def test_env_007_kid_in_tty_group(run_in_kid_env):
    rc, out, err = run_in_kid_env("id -nG kid | grep -w tty")
    assert rc == 0

def test_dir_001_apps_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -d /home/kid/apps")
    assert rc == 0

def test_dir_002_creations_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -d /home/kid/creations")
    assert rc == 0

def test_dir_003_pictures_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -d /home/kid/creations/pictures")
    assert rc == 0

def test_dir_004_programs_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -d /home/kid/creations/programs")
    assert rc == 0

def test_dir_005_games_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -d /home/kid/creations/games")
    assert rc == 0

def test_dir_006_kid_allow_bin_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -d /kid/allow/bin")
    assert rc == 0

def test_dir_007_kid_wrap_bin_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -d /kid/wrap/bin")
    assert rc == 0

def test_dir_007b_kid_bin_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -d /kid/bin")
    assert rc == 0

def test_dir_007c_kid_binary_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -x /kid/bin/kid")
    assert rc == 0

def test_dir_008_tools_exists(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -d /home/kid/tools")
    assert rc == 0

def test_dir_009_no_legacy_category_dirs(run_in_kid_env):
    rc, out, err = run_in_kid_env("test ! -d /home/kid/apps/art")
    assert rc == 0

def test_lnk_001_zshrc_is_symlink(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -L /home/kid/.zshrc")
    assert rc == 0

def test_lnk_002_zshrc_target_correct(run_in_kid_env):
    rc, out, err = run_in_kid_env("readlink /home/kid/.zshrc")
    assert rc == 0
    assert ".config/zsh/zshrc.zsh" in out

def test_lnk_003_tmux_conf_is_symlink(run_in_kid_env):
    rc, out, err = run_in_kid_env("test -L /home/kid/.tmux.conf")
    assert rc == 0

def test_lnk_004_tmux_conf_target_correct(run_in_kid_env):
    rc, out, err = run_in_kid_env("readlink /home/kid/.tmux.conf")
    assert rc == 0
    assert ".config/zsh/tmux.conf" in out
