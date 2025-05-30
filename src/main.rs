use anyhow::{Ok, Result};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::ChronoLocal;

fn main() -> Result<()> {
  // 读取环境变量日志级别，默认为 info
  let filter = EnvFilter::try_from_env("RUST_LOG").or_else(|_| Ok(EnvFilter::new("info")))?;
  // 初始化日志
  tracing_subscriber::fmt()
    .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_string())) // 设置时间格式
    .with_env_filter(filter) // 设置日志级别
    .with_line_number(true) // 显示行号
    .init();

  let mut router = v6tprouter::Router::new()?;
  loop {
    std::thread::sleep(std::time::Duration::from_secs(2));
    debug!("Checking WAN address...");
    if router.wan_check()? {
      info!("WAN has changed, updating router...");
      router.update()?;
    }
  }
}
