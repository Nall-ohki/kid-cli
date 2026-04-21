use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::{fs, path::PathBuf};
use anyhow::{Context, Result};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    pub launchers: HashMap<String, LauncherConfig>,
    pub passthroughs: HashMap<String, String>,
    pub blocks: BlockConfig,
}

impl Config {
    pub fn load(path: PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path).context("Could not read commands.toml")?;
        let config: Config = toml::from_str(&content).context("Could not parse commands.toml")?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_config() {
        let toml_str = get_default_toml();
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.launchers.contains_key("matrix"));
        assert!(config.passthroughs.contains_key("ls"));
        assert!(config.blocks.commands.contains(&"sudo".to_string()));
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LauncherConfig {
    pub binary: Option<String>,
    pub pane: String,
    pub lolcat: LolcatMode,
    pub persist: bool,
    #[serde(default)]
    pub builtin: bool,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            binary: None,
            pane: "none".to_string(),
            lolcat: LolcatMode::default(),
            persist: false,
            builtin: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum LolcatMode {
    Simple(String), // "never", "always"
    Chance { chance: f32 },
}

impl Default for LolcatMode {
    fn default() -> Self {
        LolcatMode::Simple("never".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct BlockConfig {
    pub commands: Vec<String>,
    pub message: String,
}

pub fn get_default_toml() -> &'static str {
    r#"
[launchers.matrix]
binary = "/usr/bin/cmatrix"
pane = "bottom"
lolcat = "never"
persist = false

[launchers.say]
binary = "/usr/games/cowsay"
pane = "companion"
lolcat = "never"
persist = true

[launchers.letters]
binary = "/usr/bin/figlet"
pane = "bottom"
lolcat = "never"
persist = true

[launchers.sl]
binary = "/usr/games/sl"
pane = "bottom"
lolcat = { chance = 0.25 }
persist = false

[launchers.nyan]
binary = "/usr/bin/nyancat"
pane = "bottom"
lolcat = "never"
persist = false

[launchers.ll]
binary = "/bin/ls -la"
pane = "none"
lolcat = "never"
persist = false
builtin = true

[launchers.help]
binary = "{KID} help"
pane = "popup"
lolcat = "never"
persist = false
builtin = true

[launchers.man]
binary = "/usr/bin/man"
pane = "popup"
lolcat = "never"
persist = true

[passthroughs]
ls    = "/bin/ls"
cat   = "/usr/bin/cat"
less  = "/usr/bin/less"
file  = "/usr/bin/file"
touch = "/usr/bin/touch"
echo  = "/usr/bin/echo"
whoami = "/usr/bin/whoami"
date  = "/usr/bin/date"
tmux  = "/usr/bin/tmux"
reset = "/usr/bin/reset"
bash  = "/usr/bin/bash"
base64 = "/usr/bin/base64"
iconv = "/usr/bin/iconv"
mkdir = "/bin/mkdir"
rmdir = "/bin/rmdir"
pwd   = "/bin/pwd"
wc    = "/usr/bin/wc"
head  = "/usr/bin/head"
tail  = "/usr/bin/tail"
grep  = "/bin/grep"
cal   = "/usr/bin/cal"
nano  = "/usr/bin/nano"
rm    = "/bin/rm"
mv    = "/bin/mv"
cp    = "/bin/cp"
sl    = "/usr/games/sl"

[blocks]
commands = ["sudo", "ssh", "curl", "wget", "su", "scp", "sftp", "wall"]
message = "{cmd} is not allowed... you are not going to get to use this one for a while."
"#
}
