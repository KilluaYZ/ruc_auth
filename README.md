# ruc-auth

人大校园网（`go.ruc.edu.cn`，深澜 srun_bx1 认证）命令行登录工具，Rust 实现。

自动完成 challenge 获取、XXTEA 加密、HMAC-MD5 / SHA1 校验和登录提交的全流程，支持单次登录和掉线自动重连的常驻守护模式。

## 功能特性

- **单次登录**：一条命令完成认证
- **掉线自动重连**：`serve` 模式周期检测联网状态，掉线自动重新登录
- **后台守护**：`serve --detach` 转入后台运行，带 pid 管理和日志文件
- **git config 风格配置管理**：`config get / set / init`，配置文件位于 `~/.config/ruc-auth/config.yaml`，首次使用自动初始化
- **无 OpenSSL 依赖**：使用 rustls，编译产物开箱即用

## 安装

需要 Rust 工具链（[rustup](https://rustup.rs/)，1.85+）：

```bash
git clone <repo-url> ruc-auth
cd ruc-auth
cargo build --release
```

编译产物为 `target/release/ruc-auth`，可复制到任意 PATH 目录中，例如：

```bash
sudo cp target/release/ruc-auth /usr/local/bin/
```

## 快速开始

```bash
# 1. 写入账号密码（首次执行会自动创建 ~/.config/ruc-auth/config.yaml）
ruc-auth config set username '你的学号'
ruc-auth config set password '你的密码'

# 2. 登录
ruc-auth login
```

看到 `[+] 登录成功!` 即完成认证。若返回 `ip_already_online_error`，表示当前 IP 已在线，无需重复登录。

## 命令说明

```
ruc-auth [OPTIONS] <COMMAND>

Commands:
  login   执行一次登录
  serve   常驻运行, 周期检测网络, 掉线自动重新登录
  config  读取 / 写入配置 (git config 风格)
  help    Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>  配置文件路径 (默认 ~/.config/ruc-auth/config.yaml)
  -h, --help             Print help
  -V, --version          Print version
```

### `config` — 配置管理

```bash
# 初始化配置文件（文件已存在则跳过）
ruc-auth config init

# 写入配置项
ruc-auth config set username '你的学号'
ruc-auth config set password '你的密码'
ruc-auth config set ac_id 6
ruc-auth config set interval 60

# 读取单个配置项（适合脚本使用）
ruc-auth config get username

# 列出全部配置
ruc-auth config get
```

可用配置项见下文[配置文件](#配置文件)一节，写入不存在的字段会报错并提示合法字段名。

### `login` — 单次登录

```bash
ruc-auth login
```

读取配置并执行一次完整认证流程。用户名或密码未配置时会提示先执行 `config set`。

### `serve` — 常驻守护，掉线自动重连

```bash
# 前台运行，默认每 30 秒（取配置 interval）检测一次
ruc-auth serve

# 指定检测间隔（秒）
ruc-auth serve --interval 10
ruc-auth serve -i 10

# 转入后台运行
ruc-auth serve --detach
ruc-auth serve -d
```

工作方式：

- 周期性请求 `check_url`（默认 `http://connect.rom.miui.com/generate_204`，禁用重定向）
- 收到 2xx 响应视为在线；被门户劫持、超时或请求失败视为掉线，立即触发自动重新登录
- `--detach` 会把自身重新拉起为独立进程组的后台进程：
  - pid 记录在配置目录下的 `ruc-auth.pid`（重复 detach 会检测并拒绝）
  - 运行日志写入配置目录下的 `serve.log`

查看后台运行状态与日志：

```bash
cat ~/.config/ruc-auth/ruc-auth.pid   # 查看 pid
kill $(cat ~/.config/ruc-auth/ruc-auth.pid)   # 停止后台进程
tail -f ~/.config/ruc-auth/serve.log  # 跟踪日志
```

## 配置文件

默认路径：`~/.config/ruc-auth/config.yaml`（可用全局参数 `-c/--config` 覆盖，也可通过 `XDG_CONFIG_HOME` 改变位置）。所有操作在文件不存在时都会自动初始化默认配置。

```yaml
portal: https://go.ruc.edu.cn    # 认证服务器地址
username: 'your-school-id'       # 学号 / 账号
password: 'your-password'        # 密码 (明文保存, 注意文件权限)
ac_id: '6'                       # 认证接口的 ac_id, 人大宿舍网默认 6
timeout: 15                      # HTTP 请求超时 (秒)
interval: 30                     # serve 模式的检测间隔 (秒)
check_url: http://connect.rom.miui.com/generate_204   # serve 模式的联网检测地址
```

| 字段 | 说明 | 默认值 |
|---|---|---|
| `portal` | 认证门户地址 | `https://go.ruc.edu.cn` |
| `username` | 账号（学号） | 空（必填） |
| `password` | 密码 | 空（必填） |
| `ac_id` | 深澜认证 ac_id | `6` |
| `timeout` | HTTP 超时秒数 | `15` |
| `interval` | serve 检测间隔秒数 | `30` |
| `check_url` | 联网检测地址 | `http://connect.rom.miui.com/generate_204` |

## 开机自启（可选，systemd 用户服务）

创建 `~/.config/systemd/user/ruc-auth.service`：

```ini
[Unit]
Description=RUC campus network auto login
After=network-online.target

[Service]
ExecStart=/usr/local/bin/ruc-auth serve
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
```

启用：

```bash
systemctl --user daemon-reload
systemctl --user enable --now ruc-auth.service
journalctl --user -u ruc-auth -f   # 查看日志
```

使用 systemd 时无需 `--detach`，由 systemd 负责守护。

## 开发

```bash
cargo test              # 单元测试（含与 Python 参考实现逐字节一致的加密验证向量）
cargo build --release   # 构建发布版本
```

项目结构：

```
src/main.rs    # clap CLI 定义
src/config.rs  # YAML 配置管理
src/crypto.rs  # XXTEA / 自定义 Base64 / HMAC-MD5 / SHA1
src/portal.rs  # 认证门户交互 (challenge → 登录提交)
src/serve.rs   # 掉线检测 + 自动重连 + 后台 detach
portal_login/  # 原始 Python 参考实现
```

## 常见问题

- **登录返回 `ip_already_online_error`**：该 IP 已在线，无需处理，程序按成功退出。
- **登录失败提示 challenge 获取失败**：确认当前网络在校园网环境内（能访问 `go.ruc.edu.cn`），检查账号密码是否正确。
- **serve 检测地址不通**：`check_url` 默认使用国内可达的 `connect.rom.miui.com/generate_204`，如有需要可换成任意返回 2xx 的探测地址（如 `http://captive.apple.com`）。
- **密码明文存储**：配置文件中密码为明文，建议 `chmod 600 ~/.config/ruc-auth/config.yaml` 限制读取权限。

## 许可

仅供学习与个人使用，请遵守学校网络使用规定。
