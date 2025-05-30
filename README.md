# IPv6 透明路由 (IPv6 Transparent Router)

![Version](https://img.shields.io/docker/v/lutinglt/ipv6lanrouter/latest?arch=amd64&sort=semver&color=066da5) ![Docker Pulls](https://img.shields.io/docker/pulls/lutinglt/ipv6lanrouter.svg?style=flat&label=pulls&logo=docker) ![Docker Size](https://img.shields.io/docker/image-size/lutinglt/ipv6lanrouter/latest?color=066da5&label=size) ![License](https://img.shields.io/github/license/lutinglt/ipv6lanrouter)

**新版 Docker Mac 地址无规律, 不再支持 64 位前缀的子网划分**
**(The new version of Docker Mac has irregular addresses and no longer supports subnet partitioning with 64 bit prefixes)**

**其他 IPv6 相关设置请参考 [官方文档](docs.docker.com)**
**(Please refer to other IPv6 related settings [Official documents](docs.docker.com))**

将 IPv6 地址分配给无法获得 IPv6 地址的网络, 在子网中重新下发 IPv6 地址, 并对上级路由保持透明
(Assign IPv6 addresses to networks that can't get IPv6 addresses, redistribute IPv6 addresses on the LAN, and be transparent to higher-level routing.)

## 特性 (Features)

- [x] 易于部署, 开箱即用 (Easy to deploy and out-of-the-box)
- [x] [Docker 部署](https://hub.docker.com/r/lutinglt/ipv6lanrouter)
([Docker Deploy](https://hub.docker.com/r/lutinglt/ipv6lanrouter))
2.0 需自己打包部署 (2.0 requires self packaging and deployment)
- [x] 支持自动多 LAN 口分配 (Supports automatic multi-LAN assignment)
- [x] 支持自动 WAN 口识别 (Supports automatic recognition of WAN interfaces)
- [x] 支持 WAN 口动态前缀识别并同步修改 LAN 口
(Supports recognizing dynamic prefix of WAN port and modifying LAN prefix automatically.)
- [x] 仅支持无状态 (Stateless only)
- [x] 无需 PD 服务器, 子网间路由 (No PD server required, inter-subnet routing)
- [x] 全局IPv6地址可以分配给Docker桥接网络下的容器
(Global IPv6 addresses can be assigned to containers under a Docker bridged network)

## 开始 (Getting Started)

该程序无命令行, 直接启动即可 (This program has no command line and can be launched directly)

### 构建 (Build)

需要安装 Rust (Need to install Rust)

```shell
cargo build --release
```

程序依赖 (Program dependencies) `iproute2` `radvd` `ndppd`

```shell
apt install iproute2 radvd ndppd -y
```

构建后才可以构建 Docker 镜像 (Docker images can only be built after construction)

```shell
docker build -t v6tprouter:latest .
```

仅支持 Docker 28 以上版本, 测试版本为 28.1.1 (Only supports Docker versions 28 and above, with a test version of 28.1.1)

启动配置文件参考 (Start configuration file reference) [docker-compose.yml](docker-compose.yml)

## 配置 (Configuration)

| 环境变量 (Variable)  | 描述 (Description)                                                  | 默认值 (Default) |
| --------- | ------------------------------------------------------------ | ------- |
| PREFIX_LEN | 上级路由的子网前缀长度 (Subnet prefix length of upper level routing) (WAN)    | 60      |
| RUST_LOG  | 程序打印的日志级别 (Log level printed by the program) (debug,  info) | info |
| ~~CHECK~~    | WAN 口动态前缀检测间隔, 不再支持调整, 固定为 2s (WAN port dynamic prefix detection interval, no longer supports adjustment, fixed at 2s) | 2       |
| ~~LAN_MODE~~ | ~~LAN network type~~                                           | ~~docker~~  |
| ~~MTU~~      | ~~MTU value for broadcasting when assigning IPv6 to LANs~~      | ~~0~~       |
| ~~RDNSS~~    | ~~Ditto, broadcast recursive DNS servers (Split each address with ";")~~ |   |

### 前缀长度 PREFIX_LEN ~~&& LAN_MODE~~

- 仅支持 `48-63` 的前缀 (Only prefix lengths `48-63` are supported.)

- 自动排除 WAN 口的 64 位前缀, 将其他 64 位前缀分配给 LAN 口 (Automatically exclude 64 bit prefixes from WAN ports and assign other 64 bit prefixes to LAN ports.)

- 以下功能不再支持 (The following functions are no longer supported.)

- ~~If the `PREFIXLEN` is not `64`, the WAN port address will be excluded from the subnet address pool and then the LAN port address will be assigned.~~

- ~~If the `PREFIXLEN` is `64`, the default LAN ports are all Docker bridge networks, and the IPv6 subnet address and prefix length are calculated based on the MAC address assigned to the IPv4 prefix length of the bridge network in Docker. (Linux stateless IPv6 addresses are calculated by default using EUI64).~~

> ~~If the IPv6 address is not EUI64-generated, linux can use EUI64 to calculate the IPv6 address by setting the kernel parameter `net.ipv6.conf.all.addr_gen_mode=0` `net.ipv6.conf.default.addr_gen_mode=0`.~~

- ~~If the `PREFIXLEN` is `64`, and `LAN_MODE` is set to `net` or any other value, only one LAN is supported and there is no communication between LAN port LAN and WAN port LAN.~~

| PREFIXLEN | WANIP (Example)          | Subnet Address Pool |
| --------- | ------------------------ | ------------------- |
| 56        | 2000:2000:2000:20xx::/64 | 00-ff               |
| 58        | 2000:2000:2000:20xx::/64 | 00-7f / 80-ff       |
| 60        | 2000:2000:2000:200x::/64 | 0-f                 |
| 62        | 2000:2000:2000:200x::/64 | 0-7 / 8-f           |

### ~~EXCLUDE_SUB~~

~~Addresses to exclude when assigning subnets (Supports two digits or one digit in hexadecimal only) (Split each address with ";")~~

| PREFIXLEN | Value         |
| --------- | ------------- |
| 56        | 00-ff         |
| 58        | 00-7f / 80-ff |
| 60        | 0-f           |
| 62        | 0-7 / 8-f     |

### ~~EXCLUDE_NUM1 && EXCLUDE_NUM2~~

~~Facilitates exclusion of unassigned prefixes (Supports one digit in hexadecimal only) (Split each address with ";")~~

> ~~No conflict with EXCLUDE_SUB, can be repeated.~~

| EXCLUDE_NUM1 | EXCLUDE_NUM2 | Value (EXCLUDE_SUB)     |
| ------------ | ------------ | ----------------------- |
| 0            | 0;1;2;3      | 00;01;02;03             |
| 0;1          | 0;1;2;3      | 00;01;02;03;10;11;12;13 |
