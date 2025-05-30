use std::fs::File;
use std::process::{Child, Command};

use anyhow::{Context, Result};
use tracing::{debug, info};

use crate::interface::Interface;
use crate::route::{NDPPD_CONF, RADVD_CONF, Route};
use crate::ula;

#[derive(Debug)]
pub struct Router {
  route: Route,
  radvd: Child,
  ndppd: Child,
}

impl Router {
  pub fn new() -> Result<Self> {
    info!("Initializing router...");
    let mut route = Route::new()?;
    debug!("Deleting ULA route...");
    ula::del_ula_route(&route.wan.0)?;
    info!("Starting router...");
    let (radvd, ndppd) = start_router(&mut route)?;
    Ok(Self { route, radvd, ndppd })
  }

  pub fn wan_check(&self) -> Result<bool> {
    let wan_addr = self.route.wan.1.interface.get_ipv6_global_addr()?;
    debug!("WAN address: {}", wan_addr);
    let new_wan_addr = Interface::fetch(Some(&self.route.wan.0))?.first().unwrap().get_ipv6_global_addr()?;
    debug!("New WAN address: {}", new_wan_addr);
    Ok(wan_addr != new_wan_addr)
  }

  pub fn update(&mut self) -> Result<()> {
    debug!("Killing radvd and ndppd...");
    self.radvd.kill().context("Failed to kill radvd")?;
    self.ndppd.kill().context("Failed to kill ndppd")?;

    debug!("Expiring old routes...");
    self.route.write_conf(true)?;
    let mut radvd = start_radvd()?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    radvd.kill().context("Failed to kill radvd")?;

    debug!("Cleaning old routes...");
    let old_wan_ip = self.route.wan.1.interface.get_ipv6_global_addr()?;
    debug!("Old WAN address: {}", old_wan_ip);
    debug!("Deleting old wan address...");
    Command::new("ip").args(["addr", "del", &format!("{old_wan_ip}/64"), "dev", &self.route.wan.0]).status()?;
    for (lan, lan_info) in &self.route.lans {
      let old_lan_ip = lan_info.interface.get_ipv6_global_addr()?;
      debug!("Old LAN address: {}", old_lan_ip);
      debug!("Deleting old lan address ndp proxy...");
      Command::new("ip")
        .args(["-6", "neigh", "del", "proxy", &old_lan_ip.to_string(), "dev", &self.route.wan.0])
        .status()?;
      debug!("Deleting old lan address...");
      Command::new("ip").args(["addr", "del", &format!("{old_lan_ip}/64"), "dev", &lan]).status()?;
    }

    debug!("Starting new router...");
    (self.radvd, self.ndppd) = start_router(self.route.update()?)?;
    Ok(())
  }
}

fn start_radvd() -> Result<Child> {
  let log = File::create("radvd.log").context("Failed to create radvd.log")?;
  let err_log = File::create("radvd.err.log").context("Failed to create radvd.err.log")?;
  Command::new("radvd")
    .args(["-C", RADVD_CONF, "-n"])
    .stdout(log)
    .stderr(err_log)
    .spawn()
    .context("Failed to run radvd")
}

fn start_ndppd() -> Result<Child> {
  let log = File::create("ndppd.log").context("Failed to create ndppd.log")?;
  let err_log = File::create("ndppd.err.log").context("Failed to create ndppd.err.log")?;
  Command::new("ndppd").args(["-c", NDPPD_CONF]).stdout(log).stderr(err_log).spawn().context("Failed to run ndppd")
}

fn start_router(route: &mut Route) -> Result<(Child, Child)> {
  debug!("Writing new routes...");
  route.write_conf(false)?;
  debug!("Starting radvd...");
  let radvd = start_radvd()?;
  let radvd_pid = radvd.id();
  info!(radvd_pid);
  debug!("Starting ndppd...");
  let ndppd = start_ndppd()?;
  let ndppd_pid = ndppd.id();
  info!(ndppd_pid);
  std::thread::sleep(std::time::Duration::from_secs(2));
  debug!("Adding lan ndp proxy...");
  route.add_lan_ndp_proxy()?;
  debug!("Printing route info...");
  route.route_info()?;
  Ok((radvd, ndppd))
}
