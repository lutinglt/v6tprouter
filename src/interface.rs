use std::net::Ipv6Addr;
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use tracing::{debug, info, warn};

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "lowercase")]
enum Family {
  INet,
  INet6,
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "lowercase")]
enum Scope {
  Global,
  Host,
  Link,
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct AddrInfo {
  /// 接口协议
  family: Family,
  /// 地址
  local: String,
  /// 地址作用域
  scope: Scope,
  /// 地址有效时间
  valid_life_time: u32,
  /// 地址首选时间
  preferred_life_time: u32,
}

impl AddrInfo {
  /// 检查是否为有效 IPv6 全局地址
  fn is_v6_global(&self) -> bool {
    self.family == Family::INet6
      && self.scope == Scope::Global
      && self.valid_life_time > 0
      && self.preferred_life_time > 0
      && (self.local.starts_with("2") || self.local.starts_with("3"))
  }
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct Interface {
  /// 接口名称
  pub(crate) ifname: String,
  /// 接口地址信息
  addr_info: Vec<AddrInfo>,
}

impl Interface {
  /// 获取接口信息
  /// 如果 ifname 为 None，则获取所有接口信息
  pub(crate) fn fetch(ifname: Option<&str>) -> Result<Vec<Self>> {
    let mut ip = Command::new("ip");
    ip.args(["-j", "addr", "show"]);
    if let Some(ifname) = ifname {
      ip.arg(ifname);
    }
    debug!("Running command: {:?}", ip);
    let output = ip.output().context("Failed to run ip command")?;
    let interfaces: Vec<Self> = serde_json::from_slice(&output.stdout).context("Failed to parse ip JSON")?;
    debug!("Interfaces: {:?}", interfaces);
    Ok(interfaces)
  }

  /// 获取所有接口信息 (不包含本地环回接口)
  fn fetch_interfaces() -> Result<Vec<Self>> {
    let interfaces = Self::fetch(None)?.into_iter().filter(|i| i.ifname != "lo").collect::<Vec<_>>();
    let interface_num = interfaces.len();
    info!(interface_num);
    if interface_num < 2 {
      return Err(anyhow!("Not enough interfaces"));
    }
    Ok(interfaces)
  }

  /// 获取 WAN 接口和 LAN 接口 (WAN 仅能有一个)
  pub(crate) fn fetch_wanlan() -> Result<(Self, Vec<Self>)> {
    for _ in 0..3 {
      std::thread::sleep(std::time::Duration::from_secs(2));
      let (mut wans, lans): (Vec<_>, Vec<_>) =
        Self::fetch_interfaces()?.into_iter().partition(|i| i.addr_info.iter().any(|a| a.is_v6_global()));
      debug!("WANS: {:?}", wans);
      debug!("LANS: {:?}", lans);
      match wans.pop() {
        Some(wan) => {
          if wans.is_empty() {
            return Ok((wan, lans));
          } else {
            return Err(anyhow!("Multiple WAN interfaces found"));
          }
        }
        None => {
          warn!("No WAN interface found, retrying in 2 seconds...");
          continue;
        }
      };
    }
    return Err(anyhow!("No WAN interface found"));
  }

  /// 获取接口的 IPv6 全局地址 (存储的接口信息, 不确保有效性)
  pub(crate) fn get_ipv6_global_addr(&self) -> Result<Ipv6Addr> {
    let addr = self
      .addr_info
      .iter()
      .find(|&a| a.is_v6_global())
      .context(format!("No Global IPv6 address found: {}", self.ifname))?;
    debug!("AddrInfo: {:?}", addr);
    let v6addr = addr.local.parse::<Ipv6Addr>().context(format!("Failed to parse IPv6 address: {}", self.ifname))?;
    debug!("IPv6 global address: {}", v6addr);
    Ok(v6addr)
  }

  /// 更新接口信息
  fn update_addr(&mut self) -> &mut Self {
    self.addr_info = Self::fetch(Some(&self.ifname)).unwrap().first().unwrap().addr_info.clone();
    debug!("Updated addr_info: {:?}", self.addr_info);
    self
  }

  /// 更新接口信息并获取 IPv6 全局地址
  pub(crate) fn fetch_ipv6_global_addr(&mut self) -> Result<Ipv6Addr> {
    self.update_addr().get_ipv6_global_addr()
  }
}
