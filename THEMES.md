# 自定义内置主题

## 关键字斜体开关

在 **设置 → 编辑器 → 显示 → 关键字斜体** 中可即时开启或关闭代码关键字的斜体。
首次使用时跟随主题：OneDark Pro 鲜艳斜体默认开启，其他主题按各自定义显示。
手动调整后会保存，并在切换主题、重新启动时继续使用该偏好；点击该项的
**恢复默认** 可重新跟随主题。注释、函数、类型和 Markdown 强调等其他类别仍使用主题样式。

对应配置为 `editor.keywordItalic`：`true` 强制斜体，`false` 强制正体；不写该项时跟随主题。
此设置适用于普通代码行、自动换行、粘滞标题和代码差异视图。

## OneDark Pro 鲜艳斜体

在 **设置 → 外观 → 深色主题** 中选择 **OneDark Pro 鲜艳斜体**
（英文名称为 **OneDark Pro Italic Vivid**）。外观设为深色时立即生效；
跟随系统时在系统使用深色外观时生效。选择会保存到配置：

```json
{
  "theme": "dark",
  "darkTheme": "one-dark-pro-italic-vivid"
}
```

配色以仓库根目录用户提供的 `theme.json` 为准，实现在
`crates/winc-app/src/theme/one_dark_pro.rs`。这是编译进应用的主题，运行时无需
JSON 文件，也不增加脚本或主题解析依赖。修改 `theme.json` 后仍需同步 Rust
配色并重新编译，不会自动重新加载文件。

### 配色映射

| 区域 | 来源或适配方式 |
| --- | --- |
| 编辑区 | `editor.background`：`#282C34` |
| 侧栏、窗框、状态栏 | `sideBar.background` / `statusBar.background`：`#21252B` |
| 菜单、快速打开、提示浮层 | `editorWidget.background`：`#21252B` |
| 设置卡片及分组 | 沿用 Winc 设置布局，使用编辑区底色 `#282C34` |
| 输入框 | `input.background`：`#1D1F23`，聚焦时也保留该底色 |
| 当前标签、列表选中项 | `tab.activeBackground` / `list.activeSelectionBackground`：`#2C313A` |
| 分隔线、标签及浮层边框 | 相应 `border` 配色：`#181A1F` |
| 界面文字 | `activityBar.foreground` / `list.activeSelectionForeground`：`#D7DAE0` |
| 次要文字、状态栏文字 | `statusBar.foreground`：`#9DA5B4` |
| 弱提示文字 | `titleBar.inactiveForeground`：`#6B717D` |
| 链接与强调线、实心强调区域 | 函数蓝 `#52ADF2`、`activityBarBadge.background` 的 `#4D78CC` |
| 当前行、行号 | `editor.lineHighlightBackground`、`editorLineNumber.foreground` |
| 选区 | `editor.selectionBackground`：`#67769660`，保留透明度 |
| 失焦选区 | 文件没有对应配置，使用 `editor.selectionHighlightBackground` 的 `#FFFFFF10` |
| 光标 | `editorCursor.foreground`：`#3F5DE6DA`，预合成到编辑区底色，兼容终端 RGB 协议 |
| 缩进线、空白符 | `editorIndentGuide.background`、`editorWhitespace.foreground`：`#3B4048` |
| 搜索结果 | 当前 `#42557B`、其他 `#314365`；原文件的 `editor.editor.findMatchBackground` 按当前搜索色处理 |
| 同词标记、括号配对 | `editor.wordHighlightBackground` / `editor.wordHighlightStrongBackground` |
| 滚动条 | 普通 `#4E566680`、激活 `#747D9180`，保留透明度 |
| Git 和差异 | 从 `markup.changed.diff`、`markup.inserted.diff`、`markup.deleted.diff` 取色；新增行内背景使用 `diffEditor.insertedTextBackground` |
| 终端 | 独立前景色 `#C8C8C8`；文件指定的 ANSI 黑、绿、黄、蓝原样保留，其余 12 色用同系语法色及亮色补齐 |
| 阴影、遮罩、开关、项目徽章 | 沿用 Winc 深色主题的生成规则 |

### 语法与字体

已为 Winc 的全部 21 个语法类别配置颜色，包括正文、关键字、内建名、字符串、
数字、注释、函数、装饰器、属性、变量、类型、转义、标签、标记属性、Markdown
标题/强调/加粗/链接、差异新增/删除和警告。

关键字为紫色、字符串为绿色、数字和标记属性为橙色、函数为蓝色、类型为黄色，
变量、标签和 JSON 键为红色。注释、关键字、标记属性和 Markdown 强调用斜体。
正文没有显式 `editor.foreground`，采用文件中通用代码重置色 `#AAB1C0`。

适配边界：

- Winc 使用自身词法分类，不加载 VS Code/TextMate 语法。逐语言的重叠 scope、
  运算符、参数与 `this/self` 的细分颜色不能一一对应；仅已被词法器识别的 token 会着色。
- 控制关键字与其他关键字、HTML 属性与其他标记属性合并处理，所以斜体按整个类别应用。
- 编辑器当前没有逐 token 粗体或下划线接口；Markdown 标题、加粗、链接先还原颜色。
- 多个界面部件共用 Winc 的颜色槽，无法分别设置活动栏、标题栏、状态栏悬停、
  列表焦点、普通按钮和调试状态栏的全部细分颜色。滚动条悬停与拖动共用激活色。
- 注释和普通行号保留源文件的低对比度颜色；使用高不透明度背景图片可能降低可读性。

One Dark Pro 的上游来源为 `https://github.com/Binaryify/OneDark-Pro`，
MIT 版权及许可声明保留在 `NOTICE`。

## Catppuccin Latte 与 Atom One Light

在 **设置 → 外观 → 浅色主题** 中选择 **Catppuccin Latte** 或 **Atom One Light**。
外观设为浅色时立即生效；跟随系统时在系统使用浅色外观时生效。选择会保存，
不影响另行选择的深色主题。例如：

```json
{
  "theme": "light",
  "lightTheme": "catppuccin-latte"
}
```

Atom One Light 的配置值为 `atom-one-light`。实现分别位于
`crates/winc-app/src/theme/catppuccin_latte.rs` 与
`crates/winc-app/src/theme/one_light.rs`，同样直接编译进应用，无需安装插件或新增依赖。

### 来源

| 主题 | 使用的上游版本及配色文件 | SHA-256 |
| --- | --- | --- |
| Catppuccin Latte | `@catppuccin/vscode` 3.18.1，`https://unpkg.com/@catppuccin/vscode@3.18.1/themes/latte.json` | `c5739194f3d416a0056f507e016c369c1919ca77c2ae93709aaff64deffdf771` |
| Atom One Light | `akamud/vscode-theme-onelight` 2.3.0，`https://raw.githubusercontent.com/akamud/vscode-theme-onelight/master/themes/OneLight.json` | `6628e350875e76186f3ac007680544d54a3ea132e6b542442e7704fdfac45250` |

两者均按 MIT 许可适配，版权及许可声明保留在 `NOTICE`。这是 Winc 的适配版本，
使用 Winc 自身的布局和语法分类，不等同于在 VS Code 中安装原版扩展。

### 界面与语法映射

| 区域 | Catppuccin Latte | Atom One Light |
| --- | --- | --- |
| 编辑区、当前标签 | 冷白 `#EFF1F5` | 近白 `#FAFAFA` |
| 窗框、状态栏 | `#DCE0E8` | `#EAEAEB` |
| 侧栏、菜单及浮层 | `#E6E9EF` | `#EAEAEB` |
| 设置分组 | 与编辑区同底色 | 与编辑区同底色 |
| 输入框 | `#CCD0DA` | `#FFFFFF` |
| 界面文字 | `#4C4F69` | `#383A42` |
| 强调色 / 实心按钮 | `#8839EF` | `#526FFF` / `#5871EF` |
| 列表选中项 | `#CCD0DA` | `#DBDBDC` |
| 编辑区选区 | `#7C7F934D` | `#E5E5E6` |
| 光标 | `#DC8A78` | `#526FFF` |
| 搜索结果 / 当前结果 | 蓝 `#A9DAF0` / 粉 `#E6ADBD` | `#526FFF` 的 20% / 35% 覆盖层 |
| 关键字 | 紫 `#7531CE` | 紫 `#A325A1` |
| 函数、属性 / JSON 键 | 蓝 `#1852C6` | 函数蓝 `#305BB9`、属性红 `#A53E34` |
| 字符串 | 绿 `#29681B` | 绿 `#356C35` |
| 数字 / 类型 | 橙 `#9C3D06` / 棕 `#7F5110` | 棕 `#815800` |
| 注释 | 灰紫 `#6A6D7E` | 灰 `#727377` |
| 斜体 | 注释、函数、类型、Markdown 强调 | 注释、Markdown 强调 |
| 终端前景 | `#4C4F69` | `#383A42` |

另外补齐了所有 21 个语法类别、行号、当前行、缩进线、空白符、同词高亮、括号匹配、
滚动条、拖放区域、Git 状态、差异行及词的背景色。阴影、遮罩、开关和项目徽章继续
使用 Winc 的浅色生成规则。

### 可读性与适配

- 保留上游背景和强调色，将偏浅的语法色沿原色相加深：在各自编辑区底色上，
  正文语法至少约 6:1、注释至少约 4.5:1，普通行号至少约 3:1。
  例如 Latte 字符串由 `#40A02B` 调整为 `#29681B`；One Light 注释由
  `#A0A1A7` 调整为 `#727377`。对比度指纯主题底色，叠加背景图片后可能变化。
- Latte 的 16 色终端表来自上游，偏浅的 ANSI 色加深到至少 3:1。
  One Light 上游未定义终端色表，使用同系语法色及亮色补齐。两者的亮白保留供程序铺底。
- Latte 的失焦选区使用上游同词/选区高亮的灰色透明层；滚动条悬停、拖动共用
  上游的悬停色。One Light 的当前搜索结果使用同系更深底色，未单列的 Git 状态
  配色从语法色中选取；差异行内新增保留上游青色覆盖层。
- 共享颜色槽、逐语言 scope 与粗体/下划线等限制，与上述 OneDark Pro 适配相同。
  Latte 的多个 Markdown 标题层级合并为同一红色；链接统一使用函数蓝。

开发时可运行 `cargo test -p winc-app shoot_custom_themes -- --ignored --nocapture`
生成三个主题的编辑区与设置截图，用同一段代码比较效果。
