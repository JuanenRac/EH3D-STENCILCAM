<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-STENCILCAM banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-STENCILCAM
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | 🇨🇳 <b>中文</b> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### 由锡膏层生成锡膏钢网：开口、比例检查和切割路径

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.5-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**诚实检查 - 当前真正能运行的内容：** 一个 Rust 库和一个命令行程序（`eh3d-stencilcam`），有 139 个在电脑上运行的测试：把锡膏层的焊盘变成钢网开口（缩小、倒圆、按激光切缝补偿并检查锡膏脱模），并写成 G 代码或 HP-GL。它有一个窗口（用 `--features gui` 构建），可打开锡膏层并绘制焊盘和切割路径，但窗口尚未在屏幕上看过，没有在机器上试过，也没有用大量真实文件验证过。

---

> **安全状态：** 本程序不控制任何机器，也不是安全系统。它的输出在机器上运行之前必须由人检查；这里的任何内容都没有在真实硬件上运行过。


## 📋 速览

| | |
|---|---|
| 版本 | `0.0.5` |
| 成熟度 | 脚手架 |
| 测试 | 139 |
| 技术栈 | Rust |
| 真实硬件 | 未试过：这里的任何内容都没有在机器上运行过 |

**跳转到：** [概述](#-概述) · [架构](#-架构) · [快速开始](#-快速开始) · [测试](#-测试) · [结构](#-仓库结构) · [相关项目](#-相关项目)

## 🚦 项目状态

每一行是本仓库的一个部分，灯号表示它今天做到了哪一步。内容依据仓库能够展示的证据（测试、构建、运行），而不是计划。

🟢 已完成并验证 · 🟡 可用，有已知限制 · 🟠 部分完成或尚未验证 · 🔴 未完成或不可用

| 领域 | 状态 | 在这里的含义 |
|---|---|---|
| 核心 | 🟡 | 把锡膏层的焊盘变成钢网开口（缩小、倒圆、按激光切缝补偿并检查锡膏脱模），并写成 G 代码或 HP-GL；已在电脑上用正常和损坏的输入测试，未用大量真实文件验证。 |
| 命令行 | 🟡 | `eh3d-stencilcam` 运行与库相同的函数并打印结果；每个错误都以消息和非零退出码返回。 |
| 测试 | 🟡 | 139 个测试通过（`cargo test`），包含正常情况、边界情况和损坏的输入。 |
| 安全 | 🟡 | 纯安全 Rust（`forbid(unsafe_code)`）：处理收到的文件时不会 panic，错误的值会返回带原因的错误；没有独立审查。 |
| 窗口 | 🟠 | 已编写（egui，`--features gui`）：打开或拖入锡膏层、选择材料、编辑参数、查看焊盘和切割路径（锡膏无法脱出的开口为红色）、缩放和平移、导出；其逻辑已测试，但尚未构建，也尚未在屏幕上看过。 |
| 真实硬件 | 🔴 | 未试过：这里的任何内容都没有在机器上运行过。 |
| 文档 | 🟡 | 八种语言的 README、更新日志，以及由 `docs/` 绘制的两张图。 |

## 🎯 概述

* **开口**（`src/openings.rs`）：锡膏层中每个闪现的焊盘（矩形、圆形或长圆形，无论文件单位如何一律用毫米）都是一个开口；按材料的百分比缩小，并按激光切缝缩小切割线，使孔达到要求的尺寸。底面钢网可以镜像。
* **比例**（`src/openings.rs`）：长宽比（短边除以板厚，至少 1.5）和面积比（开口面积除以孔壁面积，至少 0.66）决定锡膏能否脱模；不合格的开口会连同位置一起报告，在查看之前（或使用 `--force`）`convert` 拒绝写出。
* **路径**（`src/output.rs`、`src/profile.rs`）：倒圆的矩形、圆和长圆按最近优先的顺序组成闭合路径；激光用恒定功率的 G 代码，移动时关闭激光，或每毫米 40 单位的 HP-GL；Mylar、Kapton 和钢的配置（厚度、缩小、圆角半径和切缝）仅作为起点。
* **不会写出不完整的钢网：** 当图层中有无法变成开口的焊盘、走线或区域（多边形、宏、区域）时，除非给出 `--force`，否则 `convert` 拒绝写出文件。
* **图层从字节读取：** `--max-bytes N` 会拒绝超过上限（默认 256 MiB）的文件；非 UTF-8 的文本按 Latin-1 读取并给出警告。读取器的警告和错误连同稳定的代码一起输出，`--hide CODE[,CODE]` 可省略这些代码的提示；读取器的错误算作图层丢失了一部分，因此除非给出 `--force`，否则 `convert` 拒绝写出。开口来自 EH3D-GERBERVIEWER 的光圈转换，多边形焊盘按圆形切割（带有说明，因此除非给出 `--force`，`convert` 同样拒绝）。 `--arc-tolerance MM` 设定圆弧的跟随精度，文件过大时的拒绝信息使用系统语言显示。

## 🧩 在 EH3D 中的角色

EH3D-STENCILCAM 是 EH3D Fabrication Suite 的一个程序。它以库和命令行的形式工作，用 `--features gui` 构建时还是一个与套件其他程序外观一致的窗口。它读写简单的文件，因此可以单独使用，也可以用在其他程序之间。

## 🏛️ 架构

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-STENCILCAM 的架构：谁在使用、包含什么、依赖什么" width="100%">
</p>


本仓库各部分如何配合：谁使用它、它包含什么、它依赖什么。图由 `docs/architecture.json` 绘制并与代码核对。

## ✨ 功能

* **库加命令行：** `eh3d-stencilcam` 的每个命令都是库中的函数，套件中的其他程序无需启动进程即可调用。
* **严格的命令行：** 写错的选项（`--powr`）或多余的词会产生指明它的错误，而不是被忽略；否则程序会用默认值生成输出。
* **损坏输入的测试：** 测试还会输入空文件、乱码、巨大数字和被截断的文件，任何情况都不允许 panic。
* **不猜测：** 没有意义的值是一个说明哪个值、为什么的错误，绝不是只是看起来正确的结果。
* **没有 unsafe 代码：** 库和程序中都有 `#![forbid(unsafe_code)]`。
* **G 代码带有标准文件头，排序也更快：** 文件以生成器、单位以及图层及其 SHA-256 的注释行开头，尖角矩形没有重复的点，切割顺序用网格求得，因此十万个开口也能瞬间排好。
* **激光点亮后的等待:** `--laser-wait 秒数` 在每个 `M3` 之后写入 `G4` 等待,使激光在机头移动之前达到功率(默认为0:不等待);测试用套件的读取器重新读取G-code,找不到任何警告,也没有激光关闭时的切削。
* **每次切割都有直线引入段和引出段：** G-code 可沿与轮廓第一段相切的直线进入每条轮廓，并沿最后一段的延长线离开（默认各 0.3 mm，`--lead-in MM`、`--lead-out MM`，或 `stencil.cfg` 中的 `lead_in_mm` 和 `lead_out_mm`；0 表示不写，程序与不带引入引出段时写出的相同）。机头前往引入段起点时激光保持关闭，在该处点亮（如有要求则等待），并在引出段末端关闭。轮廓是凸的并按逆时针方向运行，因此两段都位于开口之外、保留下来的薄片上；HP-GL 中不写这两段。

## 🔒 安全与防护

| 威胁 | 防御 |
|---|---|
| 损坏、被截断或为空的文件 | 不会 panic：测试在每个位置截断有效输入并输入乱码 |
| 没有意义的数字（负数、非有限值、过大的数） | 以指出该值的错误拒绝；不会被舍入成看似合理的值 |
| 不安全的内存使用 | 带 `forbid(unsafe_code)` 的纯安全 Rust |

**没有防御的内容：** 无错误产生的结果不等于经过验证的结果。这里没有任何内容了解机器的极限，程序也没有经过独立审查。请私下报告漏洞（见 [SECURITY.md](SECURITY.md)）。

## 🔌 接口

库为 `eh3d_stencilcam`（`use eh3d_stencilcam::...`），命令行为 `eh3d-stencilcam`。命令如下；每个命令把结果打印到标准输出，把错误以非零退出码报告到标准错误。没有网络访问。

```text
EH3D-STENCILCAM/
├── src/
│   ├── browse.rs                                打开文件窗口的文件列表
│   ├── cli_args.rs                              命令行参数：每个小工具中的同一个文件
│   ├── config.rs                                设置文件：切割的引入段和引出段以及激光等待
│   ├── gui.rs                                   窗口（egui，gui 特性）：绘制状态并传递用户的操作
│   ├── lang.rs                                  窗口的文字（八种语言）
│   ├── layer.rs                                 从字节读取图层，并按代码显示读取器的提示
│   ├── lib.rs                                   库：每个命令所使用的部分
│   ├── main.rs                                  命令行
│   ├── openings.rs                              逻辑的一部分
│   ├── output.rs                                逻辑的一部分
│   ├── profile.rs                               逻辑的一部分
│   ├── shot.rs                                  窗口的截图模式（--shot）：无需屏幕即可读取和检查其参数
│   ├── viewport.rs                              图像的缩放和平移（纯数值）
│   └── window.rs                                无需屏幕的窗口状态：层、材料、参数、切割、导出和状态
├── tests/
│   ├── leads.rs                                 切割直线引入段和引出段以及设置文件的测试
│   ├── reading.rs                               文件读取测试：大小限制、非 UTF-8 文本、按代码的提示
│   ├── readme.rs                                运行 README 和 run 脚本所承诺内容的测试
│   ├── stencil.rs                               库和命令行的测试
│   ├── strict.rs                                命令行测试：它不认识的选项和词
│   └── window_cli.rs                            测试没有 gui 特性的构建仍然只是命令行
├── examples/
│   └── paste.gtp                                试用的示例
├── docs/
│   ├── architecture.json                        架构图数据
│   └── structure.json                           结构图数据
├── tools/                                       CI 和 README 检查及绘图工具
├── images/                                      横幅图以及架构和仓库结构图片
├── .github/                                     CI 与发布工作流、Issue 和 PR 模板
├── eh3d.project.json                            清单：版本、角色、成熟度和构建命令
├── Cargo.toml Cargo.lock                        工作区及其锁定的依赖
├── rust-toolchain.toml                          固定的 Rust 工具链
├── CHANGELOG.md                                 每个版本的变更
├── SECURITY.md SUPPORT.md                       如何报告漏洞及在哪里提问
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   参与贡献、行为准则和许可证
```

## ⚙️ 配置

一个小的设置文件 `stencil.cfg`（`键 = 值`，每行一个，`#` 开始注释）保存值得调整的内容：`lead_in_mm` 和 `lead_out_mm` 是每条轮廓之前的直线引入段和之后的引出段的长度（默认 0.3 mm，范围 0 到 5；0 表示不写），`laser_wait_s` 是激光点亮后的等待（默认 0，最长 60）。`eh3d-stencilcam config` 会打印带有所有默认值的文件（`examples/stencil.cfg` 就是该文件），`--config FILE` 读取文件；`--lead-in MM`、`--lead-out MM` 和 `--laser-wait 秒数` 在命令行给出同样的设置并优先于文件。未知的键、重复的键或超出范围的值会报错并指出其行号。其余内容通过命令行参数给出（见接口），每个默认值都写在命令的帮助中（`eh3d-stencilcam help`）。固定的限制是代码中的具名常量，并有测试覆盖。

## 🚀 快速开始

需要稳定版 Rust 工具链（1.75 或更新）。

```bash
cargo build --release
cargo test
cargo run --release -- check examples/paste.gtp
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/paste.gtp   # a picture of the window
```

要在不手动打开窗口的情况下截取窗口图像，请使用 `--shot FILE.png` 运行（还可用 `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`）：它按该尺寸打开，保存 PNG，不保留任何设置并自动关闭（写入文件成功时退出码为 0，否则为 1，参数错误时为 2）。

## 🧪 测试

测试套件有 139 个测试（`cargo test`，CI 在 Windows 和 Linux 上运行）。它们检查正常情况、边界情况和损坏的输入：空文件、乱码和被截断的文件绝不会 panic。CI 还运行 `cargo fmt --check`、把警告当作错误的 `cargo clippy` 以及 `python -I tools/ci_validate.py`。

## 🏗️ 构建与发布

版本号位于 `Cargo.toml` 和 `eh3d.project.json`；每个版本在 [CHANGELOG.md](CHANGELOG.md) 中都有条目。`build.bat` / `build.sh` 构建程序，`test.bat` / `test.sh` 运行测试，`run.bat` / `run.sh` 用示例运行。版本发布在 GitHub 上。

### 下载

每个版本都提供带窗口的程序，适用于 Windows（64 位和 32 位）、Linux（64 位）和 macOS（Apple silicon 和 Intel），文件名为 `<名称>-<版本>-<系统>.zip`（Windows）或 `.tar.gz`，内含程序、许可证、自述文件、图标和示例。这些文件由 GitHub Actions 构建，列在 `SHA256SUMS` 中，并使用 EH3D 的密钥签名。要检查下载的文件，请将其哈希与旁边的 `.sha256` 文件比较（`sha256sum -c`），并用旁边的 `.sig` 文件检查签名（`ssh-keygen -Y verify`）。

## 📂 仓库结构

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-STENCILCAM 的仓库结构" width="100%">
</p>

<details>
<summary>目录树的文本版</summary>

```text
EH3D-STENCILCAM/
├── src/
│   ├── browse.rs                                打开文件窗口的文件列表
│   ├── cli_args.rs                              命令行参数：每个小工具中的同一个文件
│   ├── config.rs                                设置文件：切割的引入段和引出段以及激光等待
│   ├── gui.rs                                   窗口（egui，gui 特性）：绘制状态并传递用户的操作
│   ├── lang.rs                                  窗口的文字（八种语言）
│   ├── layer.rs                                 从字节读取图层，并按代码显示读取器的提示
│   ├── lib.rs                                   库：每个命令所使用的部分
│   ├── main.rs                                  命令行
│   ├── openings.rs                              逻辑的一部分
│   ├── output.rs                                逻辑的一部分
│   ├── profile.rs                               逻辑的一部分
│   ├── shot.rs                                  窗口的截图模式（--shot）：无需屏幕即可读取和检查其参数
│   ├── viewport.rs                              图像的缩放和平移（纯数值）
│   └── window.rs                                无需屏幕的窗口状态：层、材料、参数、切割、导出和状态
├── tests/
│   ├── leads.rs                                 切割直线引入段和引出段以及设置文件的测试
│   ├── reading.rs                               文件读取测试：大小限制、非 UTF-8 文本、按代码的提示
│   ├── readme.rs                                运行 README 和 run 脚本所承诺内容的测试
│   ├── stencil.rs                               库和命令行的测试
│   ├── strict.rs                                命令行测试：它不认识的选项和词
│   └── window_cli.rs                            测试没有 gui 特性的构建仍然只是命令行
├── examples/
│   └── paste.gtp                                试用的示例
├── docs/
│   ├── architecture.json                        架构图数据
│   └── structure.json                           结构图数据
├── tools/                                       CI 和 README 检查及绘图工具
├── images/                                      横幅图以及架构和仓库结构图片
├── .github/                                     CI 与发布工作流、Issue 和 PR 模板
├── eh3d.project.json                            清单：版本、角色、成熟度和构建命令
├── Cargo.toml Cargo.lock                        工作区及其锁定的依赖
├── rust-toolchain.toml                          固定的 Rust 工具链
├── CHANGELOG.md                                 每个版本的变更
├── SECURITY.md SUPPORT.md                       如何报告漏洞及在哪里提问
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   参与贡献、行为准则和许可证
```

</details>

## 🗺️ 路线图与已知限制

**目前已知的限制**

* 窗口尚未在屏幕上看过，也不显示每个开口的比例：这由 `check` 完成。
* 没有在机器上试过，也没有用大量真实文件验证过，也没有独立审查。
* 锡膏层的宏焊盘、走线和区域不会变成开口（多边形焊盘按圆形切割），材料配置只是起点，也没有针对钢板的多道切割。

**下一步**

* 用真实的锡膏层和激光器试用，并从刀具库读取配置。
* 在屏幕上查看窗口并完善它：刀具库的配置，以及每个开口的锡膏比例显示。
* 用真实文件和套件的其他程序进行试用。

## 🌍 语言

README 和文本提供英语、西班牙语、法语、意大利语、德语、汉语、日语和欧洲葡萄牙语版本。

## 🔗 相关项目

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): Gerber、Excellon 和 G 代码的读取器
* [EH3D-PHOENIX-LASER](https://github.com/JuanenRac/EH3D-PHOENIX-LASER): 激光雕刻和切割
* [EH3D-GERBERVIEWER](https://github.com/JuanenRac/EH3D-GERBERVIEWER): Gerber 层查看器
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): CAM 程序共用的刀具库
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): G 代码到达机器之前的检查

## 📚 文档

* [更新日志](CHANGELOG.md)
* [架构与结构数据](docs/architecture.json)：绘制两张图所用的数据。

## 🤝 参与贡献与社区

* 提交拉取请求前请运行 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test` 和 `python -I tools/ci_validate.py`：必须全部通过。见 [CONTRIBUTING.md](CONTRIBUTING.md) 和[行为准则](CODE_OF_CONDUCT.md)。
* README 有八种语言：修改文字要在八个文件中同时进行，两张图由 `tools/make_architecture.py` 和 `tools/make_structure.py` 根据 `docs/architecture.json` 和 `docs/structure.json` 绘制。
* 请私下报告漏洞：[SECURITY.md](SECURITY.md)。问题、想法和报告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 许可证

软件：GPL-3.0-or-later。见 [LICENSE](LICENSE)。套件的硬件设计使用 CERN OHL-S。
