# TileNote

**为 CachyOS + Hyprland 设计的 Rust / egui Markdown 笔记磁贴。** 源码版本 0.1.0（尚未在本会话的受限容器中执行 `cargo build`）。

本项目参考用户提供的 One Dark Pro 设置页与独立磁贴示意图，并实现以下交互：

- **独立窗口**：设置和每张磁贴都分别拥有独立的原生进程、Wayland `app_id` 和 Hyprland 窗口；关闭设置不影响磁贴。
- **浮动与固定**：使用 `hyprctl` 将磁贴设置为 floating，并可通过 `pin` 跨工作区显示；使用 `alterzorder top` 调整普通窗口的堆叠位置。
- **快速呼出**：`tilenote toggle` 把所有磁贴放进/移出隐藏的特殊工作区；磁贴窗口本身无需后台守护进程，隐藏时仍在运行。
- **Markdown 双模式**：右上角眼睛按钮切换文本和渲染；预览的可点击任务 checkbox 通过 `egui_commonmark::CommonMarkViewer::show_mut` 修改 Markdown 正文。
- **支持简写**：文本中 `[] 任务`、`[ ] 任务`、`[x] 已完成` 在切换预览时自动规范为 GFM 列表 `- [ ]` / `- [x]`，并跳过围栏代码块。
- **主题**：默认 One Dark Pro；可切换 GitHub Light，不检测系统主题；6 个强调色、字号/圆角设置。
- **持久化**：输入停止约 400ms 后原子保存 `.md`，正常退出前额外保存；元数据与设置为 `.json`，无需数据库或云账户。
- **中文字体**：启动时从操作系统加载 Noto CJK/WenQuanYi 的常见安装路径；也可用 `TILENOTE_FONT=/path/to/font.ttf` 指定本机字体（不随项目分发字体）。

## 依赖与构建

在联网的 CachyOS / Arch Linux 设备上：

```bash
sudo pacman -S --needed rustup base-devel pkgconf fontconfig libxkbcommon wayland mesa
rustup default stable
cargo build --release
```

本项目锁定 `eframe = 0.33.3`、`egui_commonmark = 0.22.0`（对应 egui 0.33 系列）以减少 API 漂移。首次编译时 Cargo 会联网下载依赖。

使用 glutin/OpenGL，要求本机可运行 Wayland EGL/OpenGL。运行前需要处于 Hyprland 会话且能执行 `hyprctl`（普通桌面仍可打开界面，但不能保证固定和快捷键控制）。

安装（从源码目录执行）：

```bash
install -Dm755 target/release/tilenote "$HOME/.local/bin/tilenote"
install -Dm644 assets/io.github.tilenote.settings.desktop "$HOME/.local/share/applications/io.github.tilenote.settings.desktop"
```

请确保 `$HOME/.local/bin` 在 PATH 中，并且正确安装了中文字体（如 `noto-fonts-cjk`）。

## 命令

```bash
tilenote                  # 常规设置窗口
tilenote settings         # 同上
tilenote new "新想法"     # 新建独立磁贴
tilenote quick            # 重用名为“快速记录”的速记磁贴
tilenote toggle           # 显示/隐藏当前运行的全部磁贴
tilenote show             # 显示全部运行的磁贴
tilenote hide             # 隐藏全部运行的磁贴
tilenote list             # 查看现有笔记ID、标题
tilenote tile <id>        # 打开一张已存在的笔记
```

### Hyprland 快捷键

**旧版 hyprlang 配置语法（如 0.52）**：在 `~/.config/hypr/hyprland.conf` 中加入：

```ini
bind = SUPER, N, exec, tilenote toggle
bind = SUPER SHIFT, N, exec, tilenote new
bind = SUPER ALT, N, exec, tilenote settings
```

对于采用 Lua 配置的新版本 Hyprland，请按照你的版本将上述三条命令转换成相应的 `hl.bind` 形式，不要把旧版语法直接粘贴到 Lua 配置中。最新文档：<https://wiki.hypr.land/Configuring/Binds/>。

程序本身启动后会执行浮动、pin、置顶的窗口操作，通常无需额外窗口规则。若有严格的 compositor 管理需求，可以按 `class` 配置窗口规则：

- 磁贴 `io.github.tilenote.tile`
- 设置 `io.github.tilenote.settings`

**注意**：Wayland 常规 xdg-toplevel 不拥有真正跨所有图层的强制置顶权限；Hyprland `pin` 主要表示在各普通工作区持续显示，`alterzorder` 改变浮动窗口中的层次。全屏独占应用和层壳 surface 仍可覆盖磁贴。`tilenote toggle` 的隐藏方式依赖 Hyprland 的特殊工作区。

## 数据路径

```text
~/.config/tilenote/settings.json       # 主题/大小/默认行为
~/.local/share/tilenote/notes/*.json  # 各磁贴元数据
~/.local/share/tilenote/notes/*.md    # 各磁贴 Markdown 正文
```

上述目录遵守 `XDG_CONFIG_HOME` / `XDG_DATA_HOME`。可以使用 Syncthing、Git 或普通文件复制进行备份，跨设备编辑请避免冲突。

## 开发检查

```bash
cargo fmt --all --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

> 本交付包含完整源码、配置示例及文档；受限执行环境中没有 Rust/Cargo 工具链，**未执行 Rust 编译或图形运行测试**，如首次构建发现 API 兼容问题需继续修正。`shadow` 设置目前作为偏好数据保留，真正的阴影表现主要由 Hyprland 决定，UI 不宣称此设置已控制合成器阴影。
