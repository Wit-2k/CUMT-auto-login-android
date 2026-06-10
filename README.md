# 校园网自动登录

> 🏫 中国矿业大学校园网 Portal 认证自动登录工具
> 基于 Tauri 2 + Vue 3 构建，支持 Windows / macOS / Linux / Android

## ✨ 功能特性

- **一键登录** — 输入学号、密码，选择运营商即可完成校园网 Portal 认证
- **自动检测** — 启动时自动检测是否已联网，已联网则跳过登录
- **配置记忆** — 登录成功后自动保存账号配置，下次启动自动填充
- **断网自动登录** — 启动时检测到未联网且已有配置时，自动完成登录
- **多运营商支持** — 支持电信（@telecom）、移动（@cmcc）、联通（@unicom）
- **系统托盘** — 桌面端支持系统托盘，可后台运行
- **跨平台** — 同时支持桌面端和 Android 端

## 🖥️ 界面设计

应用窗口大小 400×480，界面简洁：

- **登录页**：学号、密码、运营商选择，一键登录
- **结果页**：显示登录成功/失败状态及服务器返回消息

## 🛠️ 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | [Tauri 2](https://tauri.app/) |
| 前端 | Vue 3 + TypeScript + Vite |
| UI 样式 | Tailwind CSS 4 |
| 后端 | Rust |
| HTTP 客户端 | reqwest (rustls-tls) |
| 系统信息 | local-ip-address, mac_address |

## 📦 项目结构

```
auto_login_tauri/
├── src/                        # Vue 前端源码
│   ├── main.ts                 # 前端入口
│   ├── App.vue                 # 根组件（页面路由逻辑）
│   ├── components/
│   │   ├── LoginPage.vue       # 登录表单页
│   │   └── StatusPage.vue      # 登录结果状态页
│   └── styles/
│       └── main.css            # 全局样式 + Tailwind CSS 入口
├── src-tauri/                  # Tauri / Rust 后端
│   ├── src/
│   │   ├── main.rs             # Rust 入口
│   │   ├── lib.rs              # Tauri Builder + 系统托盘
│   │   └── commands/
│   │       ├── auth.rs         # Portal 认证登录命令
│   │       ├── config.rs       # 配置读写命令
│   │       └── network.rs      # 网络连通性检测
│   ├── Cargo.toml              # Rust 依赖配置
│   ├── tauri.conf.json         # Tauri 应用配置
│   └── capabilities/
│       └── default.json        # 权限声明
├── index.html                  # HTML 入口
├── package.json                # Node 项目配置
├── vite.config.ts              # Vite 构建配置
└── tsconfig.json               # TypeScript 配置
```

## 🚀 快速开始

### 环境要求

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://www.rust-lang.org/) >= 1.70
- [bun](https://bun.com/) >= 1.20
- 系统依赖（Linux）：
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
  ```

维护本项目的开发者使用 bun。

### 开发调试

```bash
# 安装前端依赖
bun install

# 启动 Tauri 开发模式（热重载）
bun tauri dev
```

### 构建发布

```bash
# 构建当前平台的可执行文件
bun tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。

对于 Windows 构建，可以禁用打包，因为只有 exe 文件需要分发。

### Android 构建

调试前应当检查环境，在 PowerShell 中执行以下命令：

1.   `java --version`
2.   `adb --version`
3.   `winget list | sls "Android Studio"`
4.   `[Environment]::GetEnvironmentVariable("JAVA_HOME", "Machine") -ne $null`
5.   `[Environment]::GetEnvironmentVariable("ANDROID_HOME", "Machine") -ne $null`
6.   `[Environment]::GetEnvironmentVariable("NDK_HOME", "Machine") -ne $null`

前三条命令要有输出，后三条命令要返回 True。

```bash
bun tauri android init   # 初始化 Android 项目
bun tauri android dev    # Android 开发调试
bun tauri android build  # Android 构建 APK
```

## 📖 使用说明

1. 首次启动后，在登录页输入**学号**、**校园网密码**，选择**运营商**
2. 点击「登录」按钮，等待认证结果
3. 登录成功后，配置将自动保存到程序同目录下的 `config.json`
4. 下次启动时，若检测到未联网，将自动使用已保存的配置完成登录
5. 需要切换账号时，在结果页点击「切换/修改账号」回到登录页

## ⚙️ 配置文件

配置文件 `config.json` 位于可执行文件同目录下，格式如下：

```json
{
  "username": "01234567",
  "password": "your_password",
  "operator": "@telecom"
}
```

> 该文件格式与 [Python 版校园网登录工具](https://github.com/Wit-2k/CUMT_Auto_Log_In) 的配置文件完全兼容。

## 🔧 认证原理

应用通过 Portal 认证接口完成登录：

1. 获取本机 **IP 地址** 和 **MAC 地址**
2. 构造认证请求参数（账号、密码、IP、MAC、AC 信息等）
3. 向认证服务器 `http://10.2.5.251:801/eportal/` 发送 HTTP GET 请求
4. 解析 JSONP 响应，判断认证结果

## 📄 许可证

MIT License
