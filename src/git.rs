use std::collections::HashMap;
use std::env;
use std::process::Command;
use git2;
use errors::*;

pub trait Config {
  fn get(&self, name: &str) -> Result<String>;
  fn get_all(&self, glob: &str) -> Result<HashMap<String, String>>;
  fn set(&mut self, name: &str, value: &str) -> Result<()>;
}

pub struct GitConfig {
  namespace: String,
  repo: git2::Repository,
  config: git2::Config,
}

impl GitConfig {
  pub fn new(namespace: &str) -> Result<GitConfig> {
    let path = try!(env::current_dir().chain_err(|| ""));
    let repo = try!(git2::Repository::discover(path).chain_err(|| ""));
    let config = try!(repo.config().chain_err(|| ""));

    Ok(GitConfig {
      namespace: namespace.into(),
      repo: repo,
      config: config,
    })
  }

  pub fn auto_include(&mut self) {
    let filename = format!(".{}", self.namespace);
    let include_path = format!("../{}", filename);
    let file_exists = self.repo.workdir().map(|path| {
      let mut path_buf = path.to_path_buf();
      path_buf.push(&filename);
      path_buf.exists()
    });

    // Make sure .git-together exists
    if !file_exists.unwrap_or(false) {
      return;
    }

    if self.already_included(&include_path).unwrap_or(true) {
      return;
    }

    // TODO Figure out how to do this using git2
    let _ = Command::new("git")
      .args(&["config", "--add", "include.path", &include_path])
      .status();
  }

  fn already_included(&self, include_path: &str) -> Result<bool> {
    let local_config =
      try!(self.config.open_level(git2::ConfigLevel::Local).chain_err(|| ""));
    let entries = try!(local_config.entries(None).chain_err(|| ""));
    Ok(IntoIterator::into_iter(&entries).any(|entry| {
      entry.map(|entry| entry.value() == Some(include_path)).unwrap_or(true)
    }))
  }
}

impl Config for GitConfig {
  fn get(&self, name: &str) -> Result<String> {
    let name = format!("{}.{}", self.namespace, name);
    self.config.get_string(&name).chain_err(|| "")
  }

  fn get_all(&self, glob: &str) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    let entries = try!(self.config.entries(Some(glob)).chain_err(|| ""));
    for entry in &entries {
      let entry = try!(entry.chain_err(|| ""));
      if let (Some(name), Some(value)) = (entry.name(), entry.value()) {
        result.insert(name.into(), value.into());
      }
    }
    Ok(result)
  }

  fn set(&mut self, name: &str, value: &str) -> Result<()> {
    let name = format!("{}.{}", self.namespace, name);
    self.config.set_str(&name, value).chain_err(|| "")
  }
}
