use std::collections::BTreeMap;
use std::fs::File;
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use askama::Template;
use tracing::{debug, info, warn};

use crate::interface::Interface;

pub(crate) const RADVD_CONF: &str = "/etc/radvd.conf";
pub(crate) const NDPPD_CONF: &str = "/etc/ndppd.conf";

#[derive(Template, Debug)]
#[template(path = "radvd.conf.j2")]
struct RadvdConf<'a> {
  lan_ifname: &'a str,
  lan_prefix: String,
  expire: bool,
}

#[derive(Template, Debug)]
#[template(path = "ndppd.conf.j2")]
struct NdppdConf<'a> {
  wan_ifname: &'a str,
  lan_prefixs: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct InterfaceInfo {
  pub(crate) interface: Interface,
  pub(crate) prefix: [u16; 4],
}

impl InterfaceInfo {
  fn prefix_str(&self) -> String {
    format!("{:x}:{:x}:{:x}:{:x}", self.prefix[0], self.prefix[1], self.prefix[2], self.prefix[3])
  }
}

#[derive(Debug)]
pub(crate) struct Route {
  pub(crate) wan: (String, InterfaceInfo),
  pub(crate) lans: BTreeMap<String, InterfaceInfo>,
}

impl Route {
  /// 初始化路由信息
  pub(crate) fn new() -> Result<Self> {
    let prefix_len = match std::env::var("PREFIX_LEN") {
      Ok(prefix_len) => prefix_len.parse::<u32>().context("Failed to parse PREFIX_LEN")?,
      Err(_) => 60,
    };
    info!(prefix_len);
    if prefix_len >= 64 || prefix_len < 48 {
      return Err(anyhow!("PREFIX_LEN must be between 48 and 64(exclusive)"));
    }

    let max_subnet_num = 2u32.pow(64 - prefix_len) - 1;
    info!(max_subnet_num);
    let (wan, lans) = Interface::fetch_wanlan()?;
    info!(wan.ifname);
    for lan in &lans {
      debug!(lan.ifname);
    }
    if lans.len() > max_subnet_num as usize {
      return Err(anyhow!(
        "Too many LAN interfaces, max subnet num is {}, you can set the environment variable PREFIX_LEN(default: 60) to a smaller value to increase subnet num",
        max_subnet_num
      ));
    }

    let wan_addr = wan.get_ipv6_global_addr()?.segments();
    let mut lan_prefix: [u16; 4] = wan_addr[0..4].try_into()?;
    let lan_prefixs = lans
      .into_iter()
      .map(|lan| {
        lan_prefix[3] += 1;
        (lan.ifname.clone(), InterfaceInfo { interface: lan, prefix: lan_prefix.clone() })
      })
      .collect::<BTreeMap<_, _>>();

    Ok(Self {
      wan: (wan.ifname.clone(), InterfaceInfo { interface: wan, prefix: wan_addr[0..4].try_into()? }),
      lans: lan_prefixs,
    })
  }

  /// 更新路由信息
  pub(crate) fn update(&mut self) -> Result<&mut Self> {
    let wan_addr = self.wan.1.interface.fetch_ipv6_global_addr()?.segments();
    self.wan.1.prefix = wan_addr[0..4].try_into()?;
    info!("New WAN address: {}", self.wan.1.interface.get_ipv6_global_addr()?);
    let mut lan_prefix: [u16; 4] = wan_addr[0..4].try_into()?;
    for (_, lan_info) in &mut self.lans {
      lan_prefix[3] += 1;
      lan_info.prefix = lan_prefix.clone();
    }
    debug!("Updated LAN prefix: {:?}", self.lans);
    Ok(self)
  }

  /// 写入路由配置文件
  pub(crate) fn write_conf(&self, expire: bool) -> Result<()> {
    let mut radvd_conf = File::create(RADVD_CONF)?;
    for (lan, lan_info) in &self.lans {
      let radvd_tmpl = RadvdConf { lan_ifname: &lan, lan_prefix: lan_info.prefix_str(), expire };
      debug!("{:?}", radvd_tmpl);
      radvd_tmpl.write_into(&mut radvd_conf).context(format!("Failed to write {RADVD_CONF}"))?;
    }

    debug!(expire);
    if !expire {
      let mut ndppd_conf = File::create(NDPPD_CONF)?;
      let ndppd_tmpl = NdppdConf {
        wan_ifname: &self.wan.0,
        lan_prefixs: self.lans.values().map(|lan_info| lan_info.prefix_str()).collect(),
      };
      debug!("{:?}", ndppd_tmpl);
      ndppd_tmpl.write_into(&mut ndppd_conf).context(format!("Failed to write {NDPPD_CONF}"))?;
    }

    Ok(())
  }

  /// 添加 LAN NDP 代理
  pub(crate) fn add_lan_ndp_proxy(&mut self) -> Result<()> {
    for (lan_ifname, lan_info) in self.lans.iter_mut() {
      let mut ip = Command::new("ip");
      let lan_addr = lan_info.interface.fetch_ipv6_global_addr()?;
      ip.args(["-6", "neigh", "add", "proxy", &lan_addr.to_string(), "dev", &self.wan.0]);
      debug!("Running command: {:?}", ip);
      if ip.status()?.success() {
        debug!("Add {lan_ifname} NDP proxy: {lan_addr}");
      } else {
        warn!("Failed to add {lan_ifname} NDP proxy: {lan_addr}");
      }
    }
    Ok(())
  }

  /// 打印路由信息
  pub(crate) fn route_info(&self) -> Result<()> {
    let wan_ifname = &self.wan.0;
    let wan_inetv6 = self.wan.1.interface.get_ipv6_global_addr()?.to_string();
    let wan_prefix = self.wan.1.prefix_str();
    info!(wan_ifname);
    info!(wan_inetv6);
    info!(wan_prefix);
    for (lan_ifname, lan_info) in &self.lans {
      let lan_inetv6 = lan_info.interface.get_ipv6_global_addr()?.to_string();
      let lan_prefix = lan_info.prefix_str();
      info!(lan_ifname);
      info!(lan_inetv6);
      info!(lan_prefix);
    }
    Ok(())
  }
}
