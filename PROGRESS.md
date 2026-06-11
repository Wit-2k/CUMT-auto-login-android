# 调试进展

> 记录 Android 真机调试期间发现的 bug 表现、位置、修复和验证。不得记录真实账号密码。

## BUG-001 桌面端 Tauri Builder 被 move 后再次使用

- 表现：`cargo check --manifest-path src-tauri\Cargo.toml` 失败，报错 `use of moved value: builder`。
- 复现步骤：在仓库根目录执行 `cargo check --manifest-path src-tauri\Cargo.toml`。
- 代码位置：`src-tauri/src/lib.rs`
- 根因：`builder.setup(...)` 消费 `builder` 并返回新的 builder，但返回值只绑定在 cfg block 内部，外层继续使用已 move 的旧值。
- 修复：将桌面端 `setup` 的返回值绑定回外层 `builder`，并移除托盘回调中的 panic 型 `unwrap()`。
- 验证命令：`cargo check --manifest-path src-tauri\Cargo.toml` 通过。
- 状态：已修复，待真机回归确认无桌面端副作用。

## BUG-002 Android 配置文件写入路径不可用

- 表现：Android 上登录成功后可能无法保存 `config.json`，导致重启后无法自动填充和自动登录。
- 复现步骤：在真机上登录成功后重启应用，观察是否仍需手动输入配置。
- 代码位置：`src-tauri/src/commands/config.rs`
- 根因：配置路径使用 `std::env::current_exe()` 同目录；Android 应用的 native lib/APK 目录不适合作为可写配置目录。
- 修复：通过 Tauri `AppHandle.path().app_config_dir()` 获取应用配置目录，保存前创建目录；invoke 参数形状保持不变。
- 验证命令：`cargo test --manifest-path src-tauri\Cargo.toml` 通过；真机重启验证待执行。
- 状态：代码已修复，待真机验证。

## BUG-003 Captive Portal 可能导致联网检测误判

- 表现：未认证校园网可能拦截 `success.txt` 并返回 HTTP 200 登录页，应用误判为已联网并跳过登录。
- 复现步骤：真机连接未认证校园网后启动应用，观察是否直接显示“已连接到互联网”。
- 代码位置：`src-tauri/src/commands/network.rs`
- 根因：原实现只检查 HTTP 状态码是否成功，没有校验响应正文是否为 Firefox 探测端点的 `success`。
- 修复：联网检测同时要求状态码成功且响应正文 trim 后等于 `success`。
- 验证命令：`cargo test --manifest-path src-tauri\Cargo.toml` 通过；真机场景待执行。
- 状态：代码已修复，待真机验证。

## BUG-004 Base64 成功消息未参与成功判定

- 表现：Portal 若把“认证成功”放在 Base64 编码的 `msg` 内，界面可能显示解码后的成功消息但仍判定失败。
- 复现步骤：对 JSONP 响应 `{"result":"0","msg":"6K6k6K+B5oiQ5Yqf"}` 运行解析逻辑。
- 代码位置：`src-tauri/src/commands/auth.rs`
- 根因：原实现解码了 `msg`，但成功判断仍只检查原始 `message.contains("认证成功")` 和 `result == "1"`。
- 修复：抽出 JSONP/消息解析函数，并让解码后的消息也参与成功判定。
- 验证命令：`cargo test --manifest-path src-tauri\Cargo.toml` 通过。
- 状态：已修复。

## BUG-005 Android MAC 获取失败会阻塞登录

- 表现：Android 无法读取 `/sys/class/net/.../address` 时，登录在请求 Portal 前直接失败。
- 复现步骤：真机点击登录，若系统限制 MAC 读取，界面显示“无法获取MAC地址”类错误。
- 代码位置：`src-tauri/src/commands/auth.rs`
- 根因：现代 Android 对 MAC 访问有限制，原实现把读取失败作为硬错误返回。
- 修复：移动端改为尽力读取并过滤占位 MAC；读取不到时传空 MAC 继续请求 Portal，让服务器返回真实认证结果。
- 验证命令：`cargo test --manifest-path src-tauri\Cargo.toml` 覆盖占位 MAC 过滤；真机登录验证待执行。
- 状态：代码已修复，待真机验证。

## BUG-006 移动数据导致启动假成功

- 表现：使用数据网络时，未连接 WLAN 或已连接 WLAN 但未完成校园网登录，启动后仍显示登录成功页。
- 复现步骤：真机仅开启移动数据，启动应用；或连接未认证 WLAN 且保留移动数据，启动应用。
- 代码位置：`src-tauri/src/commands/network.rs`、`src-tauri/src/commands/auth.rs`
- 根因：联网检测只验证公网探测地址；Android 在 WLAN 不可用或未认证时会走移动数据，因此公网探测成功不能代表校园网已登录。
- 修复：Android 本机 IP 只从 `wlan0` 获取；联网检测必须先拿到 WLAN IPv4，并把 HTTP client 绑定到该 WLAN IP。实现已从外部 `ip` 命令改为进程内 `getifaddrs` 枚举，详见 BUG-009。
- 验证命令：`cargo test --manifest-path src-tauri\Cargo.toml` 通过；`cargo check --manifest-path src-tauri\Cargo.toml --target aarch64-linux-android` 通过；`npx tauri android build --debug --target aarch64 --apk --ci` 通过；真机仅移动数据且 `wlan0` 无 IPv4 时启动停留登录页。
- 状态：代码已修复；连接未认证 WLAN 且保留移动数据的场景待人工复测。

## BUG-007 Portal 请求被蜂窝网 10/8 路由劫持

- 表现：无论是否连接 WLAN，手动登录都提示“登录异常：网络请求失败，认证服务器无响应。”
- 复现步骤：真机点击登录；ADB 上 `ip route` 显示 `10.0.0.0/8 dev ccmni1`，`ip -4 route get 10.2.5.251` 走 `ccmni1`。
- 代码位置：`src-tauri/src/commands/auth.rs`
- 根因：手机蜂窝网分配了 `10.x.x.x` 地址并有 `10.0.0.0/8` 路由，Portal 服务器 `10.2.5.251` 请求被发往蜂窝网而不是校园 WLAN。
- 修复：Portal 登录请求使用从 `wlan0` 获取的 IPv4 作为 `reqwest::ClientBuilder::local_address`，强制绑定 WLAN 源地址；`wlan0` 地址获取已从外部 `ip` 命令改为进程内 `getifaddrs` 枚举。
- 验证命令：`ip -4 route get 10.2.5.251` 确认设备蜂窝网会劫持 `10.0.0.0/8`；`cargo check --manifest-path src-tauri\Cargo.toml --target aarch64-linux-android` 通过；安装新 APK 后启动应用，logcat 未再出现 `netlink_route_socket`、`/system/bin/ip` 或 WLAN 探测错误。
- 状态：代码已修复；连接校园 WLAN 后的真实账号登录待人工复测。

## BUG-008 APK 体积过大

- 表现：debug arm64 APK 约 163.49 MB，release unsigned APK 基线约 8.87 MB；release 包内 `lib/arm64-v8a/libcumt_auto_login_lib.so` 约 6.70 MB，是主要体积来源。
- 复现步骤：执行 `npx tauri android build --target aarch64 --apk --ci` 后检查 `src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk`，并用 Zip 条目统计 APK 分项大小。
- 代码位置：`src-tauri/Cargo.toml`、`src-tauri/src/commands/auth.rs`、`src-tauri/src/commands/network.rs`、`src-tauri/src/commands/http.rs`、`src-tauri/tauri.conf.json`、`scripts/patch-android-gradle.mjs`
- 根因：debug APK 包含 native 调试符号，不适合作为分发体积基线；release APK 的主要成本来自应用直接引入的 `reqwest 0.12`/TLS/async HTTP 栈和 Android 资源未启用 shrink。`app-universal-release-unsigned.apk` 实测只包含 `arm64-v8a`，不是多 ABI 膨胀。
- 修复：release 体积改以 APK 分项统计为准；移除未使用的 `tauri-plugin-shell` 和 shell capability；桌面端才启用 `tray-icon`，Android 基础 Tauri 依赖关闭默认 feature 并只保留 `wry`；移除直接 `regex` 依赖并改为手写 JSONP 切片；用 `std::net::TcpStream` + `socket2` 实现 HTTP-only GET，保留 WLAN 源地址绑定、超时、文本响应和 chunked 响应解析；移除直接 `reqwest 0.12`；移除不必要的 `staticlib` crate type；通过 `scripts/patch-android-gradle.mjs` 在生成的 Android release build block 中启用 `isShrinkResources = true`。
- 验证命令：`npm run test` 通过；`cargo test --manifest-path src-tauri\Cargo.toml` 通过，23 个 Rust 单测；`npm run build` 通过；`cargo check --manifest-path src-tauri\Cargo.toml` 通过；`cargo check --manifest-path src-tauri\Cargo.toml --target aarch64-linux-android` 通过；`npx tauri android build --target aarch64 --apk --ci` 通过。
- 体积结果：release APK 从 9,304,797 bytes（8.87 MB）降至 5,500,538 bytes（5.25 MB），减少 3,804,259 bytes，约 40.88%；native `.so` 从 6,698,984 bytes 降至 3,600,288 bytes；启用 resource shrink 后 `resources.arsc` 从 1,012,644 bytes 降至 603,392 bytes，XML 从 630 个降至 345 个，图片从 217 个降至 171 个。
- 状态：代码与 release 构建已验证；HTTP-only 客户端的真实 Portal 登录和已认证 WLAN 联网检测需在 Android 真机校园网环境复测。

## BUG-009 Android 应用沙盒拒绝外部 `ip` 命令的 netlink 访问

- 表现：真机已连接 `CUMT_Stu`，`adb shell ip -4 addr show wlan0` 能看到 `10.3.67.230/16`，但应用内点击登录仍提示“登录异常：无法检查 WLAN 网络，请确认已连接校园网WiFi。”
- 复现步骤：连接 WLAN，填入正确账号密码后点击登录；或在 adb 中执行 `adb shell run-as com.cumt.autologin /system/bin/ip -4 addr show wlan0`。
- 代码位置：`src-tauri/src/commands/auth.rs`
- 根因：Android 普通应用沙盒执行 `/system/bin/ip` 时无法绑定 `netlink_route_socket`，logcat 出现 `avc: denied { bind } ... app=com.cumt.autologin`；该命令在 `adb shell` 下可用，但在 App 进程上下文不可用。
- 修复：Android 分支不再启动 `/system/bin/ip`；新增 `libc::getifaddrs` 枚举 IPv4 接口，只选择接口名为 `wlan0` 且非 unspecified/loopback/link-local 的地址，再继续用于 Portal 参数和 `reqwest::ClientBuilder::local_address`。
- 验证命令：`cargo test --manifest-path src-tauri\Cargo.toml` 通过；`cargo check --manifest-path src-tauri\Cargo.toml --target aarch64-linux-android` 通过；`npx tauri android build --debug --target aarch64 --apk --ci` 通过；`adb install -r ...\app-universal-debug.apk` 成功；启动后筛选 logcat，未匹配 `netlink_route_socket`、`Cannot bind netlink`、`/system/bin/ip`、`无法检查WLAN` 或 `未检测到WLAN`。
- 状态：代码已修复，启动路径已在真机回归；真实 Portal 登录结果待用户用正确账号密码复测。

## BUG-010 Android 已认证 WLAN 启动仍停留登录页

- 表现：WLAN 已连接且系统网络栈显示 `CUMT_Stu` 已 `VALIDATED`，启动应用仍显示登录页。
- 复现步骤：真机连接并认证 `CUMT_Stu` 后启动应用。
- 代码位置：`src-tauri/src/commands/network.rs`
- 根因：联网检测只请求 `http://detectportal.firefox.com/success.txt` 并要求正文为 `success`；真机系统网络栈使用 `http://connect.rom.miui.com/generate_204` 验证，已认证网络返回 204，未认证/Portal 拦截时历史记录返回 200 和 Portal 页面。
- 修复：改为多探测端点策略：优先检查 `http://connect.rom.miui.com/generate_204` 和 `http://www.gstatic.com/generate_204`，这类端点必须返回 204 且空正文才算在线；保留 Firefox `success.txt` 作为 fallback，仍要求正文精确为 `success`。
- 验证命令：`cargo test --manifest-path src-tauri\Cargo.toml` 通过；`cargo check --manifest-path src-tauri\Cargo.toml --target aarch64-linux-android` 通过；`npx tauri android build --debug --target aarch64 --apk --ci` 通过；`adb install -r ...\app-universal-debug.apk` 后启动应用，`uiautomator dump` 显示“✅ 已连接到互联网，无需登录。”。
- 状态：已修复并通过真机已认证 WLAN 启动验证。

## BUG-011 Portal 已登录/成功响应解析过窄

- 表现：WLAN 连接成功后，手动输入正确账号密码显示“登录失败”，服务器消息为空。
- 复现步骤：真机在已连接 WLAN 状态下输入正确账号密码点击登录。
- 代码位置：`src-tauri/src/commands/auth.rs`
- 根因：成功判定只接受字符串 `result == "1"` 或消息包含“认证成功”，如果 Portal 返回数字 `result: 1`、布尔成功字段、或“已在线/已登录”类消息，会被误判失败；空 `msg` 也会导致界面缺少可诊断信息。
- 修复：登录解析接受字符串/数字/布尔成功字段，接受“认证成功/登录成功/已在线/已登录/already online/already logged/success”等成功消息；消息字段兼容 `msg/message/ret_msg/error_msg/errorMessage/desc/info/data`；服务端消息为空时返回 `result/success/ok/ret_code/code` 摘要，避免 UI 只显示空消息。
- 验证命令：`cargo test --manifest-path src-tauri\Cargo.toml` 通过，覆盖数字 `result: 1`、布尔 `success: true`、已在线消息和空失败消息摘要。
- 状态：代码已修复并有单测覆盖；真实手动登录响应待用户用正确账号密码复测。

## BUG-012 已联网时重复提交相同账号触发 Portal 重复登录失败

- 表现：WLAN 已连接成功后，点击“切换/修改账号”，输入与已保存配置相同的账号密码，界面显示“登录失败”，服务器消息为“服务器未返回消息 (result=\"0\",ret_code=\"2\")”。
- 复现步骤：真机已认证 `CUMT_Stu` 后进入状态页，点击“切换/修改账号”，保持保存的账号、密码、运营商不变，点击登录。
- 代码位置：`src/App.vue`
- 根因：前端手动登录总是调用 `perform_login`，即使当前配置已经在线且输入与保存配置完全相同；Portal 对重复登录返回 `result=0, ret_code=2` 且无消息，导致界面显示失败。
- 修复：手动登录前加载已保存配置；如果账号、密码、运营商与输入完全相同，则先调用 `check_online` 验证连接状态；已在线时直接显示“已连接到互联网，无需重复登录”，不再调用 `perform_login`。启动自动登录不套用该跳过逻辑，保持原有离线自动登录行为。
- 验证命令：`npm run test` 通过，新增“相同配置且在线时跳过 `perform_login`”单测；`npm run build` 通过；`cargo test --manifest-path src-tauri\Cargo.toml` 通过；`cargo check --manifest-path src-tauri\Cargo.toml` 通过；`cargo check --manifest-path src-tauri\Cargo.toml --target aarch64-linux-android` 通过；`npx tauri android build --debug --target aarch64 --apk --ci` 通过；覆盖安装 APK 后在真机点击“切换/修改账号”再点“登 录”，UI dump 显示“✅ 已连接到互联网，无需重复登录。”。
- 状态：已修复并通过真机重复登录场景验证。

## BUG-013 首次登录成功后被重复请求失败覆盖

- 表现：WLAN 未登录时输入正确账号密码，界面先显示“登录成功 / 认证成功”，一两秒后变为“登录失败”，服务器消息为“服务器未返回消息 (result=\"0\", ret_code=\"2\")”。用户截图 `Screenshot_2026-06-10-22-36-12-589...jpg` 与 `Screenshot_2026-06-10-22-36-17-577...jpg` 可复现该先成功后失败的顺序。
- 复现步骤：真机连接未认证 `CUMT_Stu`，应用内已有保存配置或启动自动登录仍在进行时，手动输入同一账号密码点击登录。
- 代码位置：`src/App.vue`、`src-tauri/src/commands/auth.rs`
- 根因：启动阶段会在 `check_online == false` 且存在保存配置时自动调用 `perform_login`；用户几乎同时手动提交相同账号会产生第二个相同 Portal 请求。Portal 对重复认证返回 `result=0, ret_code=2` 且消息为空，后完成的重复请求会覆盖先完成的成功状态。
- 修复：前端为 Portal 登录增加同账号在途请求合并，若相同账号、密码、运营商的登录请求已在进行中，后续提交等待同一个请求而不再发第二个 `perform_login`；同时增加登录 token，旧请求不能覆盖新请求状态。Rust 解析层把 `ret_code=2` / `code=2` 识别为“已在线，无需重复登录”，避免重复认证响应被展示成失败。
- 验证命令：`npm run test` 通过，新增“自动登录在途时手动提交同账号只调用一次 `perform_login`”单测；`npm run build` 通过；`cargo test --manifest-path src-tauri\Cargo.toml` 通过，新增 `ret_code=\"2\"` 已在线解析单测；`cargo check --manifest-path src-tauri\Cargo.toml` 通过；`cargo check --manifest-path src-tauri\Cargo.toml --target aarch64-linux-android` 通过；`npx tauri android build --debug --target aarch64 --apk --ci` 通过；覆盖安装 APK 后已认证 WLAN 启动回归正常且 logcat 未匹配已知错误。
- 状态：代码已修复并通过竞态单测/构建/安装验证；未认证 WLAN 的真实首次登录场景待用户复测。

## BUG-014 release APK 未签名导致手机解析安装失败

- 表现：手机文件管理器点击 release APK 提示“解析软件包时出现问题。（33）详细信息：packageInfo is null”。
- 复现步骤：将 `src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk` 复制到手机并点击安装。
- 代码位置：`package.json`、`scripts/sign-android-release-debug.mjs`、`README.md`
- 根因：`npx tauri android build --target aarch64 --apk --ci` 生成的 release APK 文件名为 `app-universal-release-unsigned.apk`，未包含 APK 签名块；`apksigner verify --verbose` 报 `DOES NOT VERIFY` 和 `Missing META-INF/MANIFEST.MF`，Android/MIUI 文件安装器无法解析为可安装包。
- 修复：新增 `scripts/sign-android-release-debug.mjs`，使用 Android SDK 最新 `apksigner` 和本机 Android debug keystore 对 unsigned release APK 签名，输出 `app-universal-release-debugsigned.apk`；新增 `npm run android:sign:release` 和 `npm run android:build:signed`；README 标明可安装 release 调试包路径。该 debug 签名仅用于真机调试，不用于正式分发。
- 验证命令：`npm run android:sign:release` 通过并输出 `app-universal-release-debugsigned.apk`；`apksigner verify --verbose ...\app-universal-release-debugsigned.apk` 显示 v2/v3 签名通过；`adb install -r src-tauri\gen\android\app\build\outputs\apk\universal\release\app-universal-release-debugsigned.apk` 成功；`adb shell monkey -p com.cumt.autologin -c android.intent.category.LAUNCHER 1` 成功注入启动事件。
- 状态：已修复并通过签名校验、adb 安装和启动冒烟验证；用户可再用文件管理器点击 `app-universal-release-debugsigned.apk` 复测原始解析失败路径。

## 环境与构建基线

- `npm run test`：通过，1 个测试文件 / 6 个测试。
- `cargo test --manifest-path src-tauri\Cargo.toml`：通过，20 个 Rust 单测。
- `npm run build`：通过。
- `cargo check --manifest-path src-tauri\Cargo.toml`：通过。
- `cargo check --manifest-path src-tauri\Cargo.toml --target aarch64-linux-android`：通过。
- `npx tauri android build --debug --target aarch64 --apk --ci`：在临时使用 Machine 级 `JAVA_HOME` 后通过，APK 位于 `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`。
- `adb devices -l`：已连接 `99GMUCOF9HOZDE79`，设备 `arm64-v8a` / Android 14。
- 真机仅移动数据验证：安装新 APK、清空应用数据后启动，UI dump 显示登录页；假账号手动登录后显示“无法检查WLAN网络，请确认已连接校园网WiFi。”。
- 真机 WLAN 启动回归：覆盖安装新 APK 并启动，logcat 未匹配 `netlink_route_socket`、`Cannot bind netlink`、`/system/bin/ip`、`无法检查WLAN` 或 `未检测到WLAN`。
- 真机已认证 WLAN 启动验证：覆盖安装新 APK 并启动，UI dump 显示“✅ 已连接到互联网，无需登录。”。
- 真机重复登录验证：状态页点击“切换/修改账号”后保持已保存配置不变并点击登录，UI dump 显示“✅ 已连接到互联网，无需重复登录。”。
