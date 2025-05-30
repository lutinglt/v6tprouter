use std::process::Command;

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, warn};

#[derive(Debug, Deserialize, Clone)]
struct RouteList {
  /// 目标地址
  dst: String,
  /// 网关地址
  gateway: Option<String>,
}

/// 删除 ULA 路由, 避免错误路由导致子网无法访问
pub(crate) fn del_ula_route(ifname: &str) -> Result<()> {
  let output = Command::new("ip")
    .args(["-j", "-6", "route", "show", "dev", ifname])
    .output()
    .context("Failed to run ip command")?;
  debug!("Ip route JSON: {}", String::from_utf8_lossy(&output.stdout));
  let route_lists: Vec<RouteList> = serde_json::from_slice(&output.stdout).context("Failed to parse ip JSON")?;
  debug!("RouteList: {:?}", route_lists);

  for route_list in &route_lists {
    if let Some(gateway) = &route_list.gateway {
      if route_list.dst == "default" && gateway.starts_with("fd") {
        let mut ip = Command::new("ip");
        ip.args(["-6", "route", "del", "default", "via", gateway, "dev", ifname]);
        debug!("Running command: {:?}", ip);
        if ip.status()?.success() {
          debug!("Delete route: default via {} dev {}", gateway, ifname);
        } else {
          warn!("Failed to delete route: default via {} dev {}", gateway, ifname);
        }
      }
    }
  }
  Ok(())
}
