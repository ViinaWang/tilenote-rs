# TileNote

[![CI](https://github.com/ViinaWang/tilenote-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ViinaWang/tilenote-rs/actions/workflows/ci.yml)

**Hyprland / Wayland 桌面上的 Rust + egui 浮动 Markdown 笔记磁贴。**

每张笔记都是一个独立的无边框小窗口：可以浮动、置顶、跨工作区常驻，随手记录、随手隐藏。笔记正文是纯 Markdown 文件，保存在本地，不依赖数据库和云账户。

## 功能

- **独立磁贴窗口**：设置窗口和每张磁贴分别是独立进程，拥有各自的 Wayland `app_id`（`io.github.tilenote.tile` / `io.github.tilenote.settings`），关闭设置不影响磁贴。
- **浮动与置顶**：磁贴启动后自动通过 `hyprctl` 设置浮动、`pin` 跨工作区常驻、`alterzorder` 置顶，无需手动平铺管理。
- **笔记库双视图**：设置窗口首页是全部笔记的「画布」——可拖动重排的迷你磁贴地图；也可切到「列表」三栏视图（分类 → 笔记列表 → Markdown 详情）按分类、搜索快速定位。
- **一键呼出**：`tilenote toggle` 把所有磁贴收进/移出隐藏的特殊工作区，磁贴隐藏期间仍在运行。
- **Markdown 双模式**：磁贴右上角按钮切换编辑 / 渲染预览；预览中可点击的 checkbox 会直接写回 Markdown 正文。
- **任务简写**：`[] 任务`、`[ ] 任务`、`[x] 已完成` 在切换预览时自动规范为 GFM 的 `- [ ]` / `- [x]`（跳过围栏代码块，可在设置中关闭）。
- **主题**：One Dark / One Dark Dim / One Light 三主题、6 种强调色、密度 / 字号 / 圆角可调，设置项支持搜索跳转。
- **圆角磁贴**：默认使用带柔和阴影的透明圆角窗口（需合成器支持 alpha；异常时可在「磁贴 → 圆角与阴影窗口」关闭）。
- **本地持久化**：输入停止后按设置的防抖间隔原子写入 `.md`，元数据与设置存为 `.json`。
- **中文显示**：启动时通过 fontconfig 加载系统简体中文字体（Noto CJK SC 等）；也可用 `TILENOTE_FONT=/path/to/font.ttc:2` 手动指定字体与 face。项目本身不分发字体文件。

## 安装

### 从 CI 产物安装

每次推送到 `main` 后，[Actions](https://github.com/ViinaWang/tilenote-rs/actions) 会编译 Linux x86-64 二进制并上传为 artifact（`tilenote-linux-x86_64`）。下载解压后：

```bash
install -Dm755 tilenote "$HOME/.local/bin/tilenote"
install -Dm644 assets/io.github.tilenote.settings.desktop \
    "$HOME/.local/share/applications/io.github.tilenote.settings.desktop"
```

请确保 `$HOME/.local/bin` 在 PATH 中，且系统安装了简体中文字体（如 `noto-fonts-cjk`）。

### 从源码构建

在 CachyOS / Arch Linux 上：

```bash
sudo pacman -S --needed rustup base-devel pkgconf fontconfig libxkbcommon wayland mesa
rustup default stable
cargo build --release
```

依赖锁定 `eframe = 0.33.3`、`egui_commonmark = 0.22.0`（egui 0.33 系列）。渲染基于 glutin/OpenGL，需要 Wayland EGL 可用。运行时需要 Hyprland 会话与 `hyprctl`（普通桌面也能打开界面，但浮动、置顶与呼出控制不可用）。

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

## Hyprland 快捷键

全局快捷键由 Hyprland 提供，TileNote 不抢占系统按键。下面以 `SUPER + M` 为例（`M` 只示例键位，请换成未被你占用的按键）：

**Lua 配置（Hyprland 0.53+，见 [examples/hyprland.lua](examples/hyprland.lua)）**

```lua
hl.bind("SUPER + M",          hl.dsp.exec_cmd("tilenote toggle"))
hl.bind("SUPER + SHIFT + M",  hl.dsp.exec_cmd("tilenote new"))
hl.bind("SUPER + ALT + M",    hl.dsp.exec_cmd("tilenote settings"))
```

**旧版 hyprlang 配置（见 [examples/hyprland-legacy.conf](examples/hyprland-legacy.conf)）**

```ini
bind = SUPER, M, exec, tilenote toggle
bind = SUPER SHIFT, M, exec, tilenote new
bind = SUPER ALT, M, exec, tilenote settings
```

最新绑定文档：<https://wiki.hypr.land/Configuring/Binds/>。

程序启动后会自行完成浮动、pin、置顶等窗口操作，通常无需额外窗口规则。如果个别发行版上磁贴仍以平铺方式打开，可以加一条窗口规则兜底（旧版语法）：

```ini
windowrule = float, class:^(io\.github\.tilenote\.tile)$
```

也可以按 `class` 做更细的 compositor 管理（磁贴 `io.github.tilenote.tile`，设置 `io.github.tilenote.settings`）。

> Wayland 普通窗口没有跨所有图层的强制置顶权限：Hyprland 的 `pin` 表示在普通工作区持续显示，全屏应用和层壳 surface 仍可覆盖磁贴；`tilenote toggle` 的隐藏依赖 Hyprland 特殊工作区。

## 数据路径

```text
~/.config/tilenote/settings.json       # 主题/大小/默认行为
~/.local/share/tilenote/notes/*.json  # 各磁贴元数据
~/.local/share/tilenote/notes/*.md    # 各磁贴 Markdown 正文
```

遵守 `XDG_CONFIG_HOME` / `XDG_DATA_HOME`。可用 Syncthing、Git 或文件复制备份；多设备同时编辑请自行避免冲突。

## 开发

```bash
cargo fmt --all --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

## License

[MIT](LICENSE)
