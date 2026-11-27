# 文本处理插件（API v1）

本 fork 提供基于 Rhai 1.27.0 / Grain 的本地文本插件。用户无需安装 Rhai、Node.js 或 Python。
脚本接收选中文本，返回替换后的字符串；不选择文字时处理全文。

## 安装与使用

1. 编译并运行本分支的 Winc，例如 `rtk cargo run -- --new-instance`。
   `--new-instance` 避免把启动请求交给已经运行的旧版 Winc。
2. 打开命令面板，搜索“插件”。
3. 执行“插件：导入目录”，选择 Winc-plugin 仓库中的 `plugins/remove-duplicate-lines`；也可以使用
   “插件：导入插件包或清单”，选择该目录中的 `plugin.json` 或 ZIP。
4. 打开文本文件，选择需要处理的内容，在命令面板搜索“去除重复行”并执行。
5. 使用撤销可以一次还原此次操作；重做同样有效。

“设置 → 快捷键”中也会出现插件命令，可以按现有方式绑定组合键。首次安装不抢占任何快捷键。
禁用或卸载不会删除已经保存的插件快捷键；重新安装同一标识的命令后可以继续使用。

使用“插件：管理插件”查看插件名称、版本及状态，并启用、禁用或卸载插件。
已安装脚本出现语法错误时，会显示“无法加载”，仍然可以禁用、修复后重新加载或卸载。
同一标识的插件不会被静默覆盖；更新时先卸载旧包，再导入新包。

插件导入后复制到插件目录，关闭重开后自动加载。常规安装使用配置目录下的
`plugins/<插件标识>/`（Windows 为 `%APPDATA%/winc/plugins/`）。便携版使用
`winc.exe` 旁的 `plugins/<插件标识>/`，与 `data` 同级：

```text
winc.exe
data/
plugins/
├── winc.remove-duplicate-lines/
│   ├── plugin.json
│   └── main.rhai
├── winc.remove-empty-line/
│   ├── plugin.json
│   └── main.rhai
├── winc.sort-json-keys/
│   ├── plugin.json
│   └── main.rhai
└── winc.chinese-conversion/
    ├── plugin.json
    ├── main.rhai
    └── dictionaries/
```

旧版升级时，将 `data/plugins` 中的插件目录移入同级的 `plugins`，同名插件保留自己
修改的版本，再执行“插件：重新加载”。更新便携版时保留 `data` 和 `plugins` 两个目录。

修改原来的导入目录不会影响已安装副本；开发时修改已安装脚本，随后执行
“插件：重新加载”。禁用状态保存在包目录的 `.disabled` 文件中。

## 独立插件仓库

插件源码、字典和 ZIP 在 [Winc-plugin](https://github.com/linmii/Winc-plugin) 独立维护。
Winc 保留插件运行时和宿主接口，便携包不再预装插件。已有安装中的插件目录继续有效。

目前提供去除重复行、删除空白行、JSON / JSONC 按键名排序、简繁互转。
具体用法和选项以 Winc-plugin 的说明为准。

在 Winc-plugin 仓库运行 `pwsh -NoProfile -File scripts/package.ps1` 可打包全部插件，
产物位于该仓库的 `dist/`；也可以下载其 GitHub Actions 的 `Winc-plugins` 构建产物。
先解压 Actions 产物，再通过“插件：导入插件包或清单”导入其中单个插件 ZIP。
只修改插件时，在 Winc-plugin 提交和打包即可；新增宿主接口时才需要修改 Winc。

## 插件格式

```text
my-plugin/
├── plugin.json
└── main.rhai
```

ZIP 内直接放 `plugin.json`、脚本和资源文件，不要额外包一层目录。导入时只复制清单和被声明的脚本、资源。

```json
{
  "id": "my.text-tools",
  "name": "我的文本工具",
  "version": "1.0.0",
  "api_version": 1,
  "commands": [
    {
      "id": "uppercase",
      "title": "转换为大写 / Uppercase",
      "script": "main.rhai",
      "options": { "prefix": "" }
    }
  ]
}
```

`main.rhai`：

```rhai
// 最后一个表达式必须返回字符串。
options.prefix + text.to_upper()
```

清单和脚本使用 UTF-8。插件和命令标识应以小写字母开头，使用小写字母、数字、点或连字符，
最多 64 字节。标识一旦用于发布应保持稳定，快捷键配置使用
`plugin:my.text-tools/uppercase`。不同插件可以使用相同的局部命令名。
一个包最多提供 32 个命令，可以复用同一个脚本。

## 脚本接口

每次调用均有全新的作用域，以下变量由宿主提供：

| 变量 | 类型 | 含义 |
| --- | --- | --- |
| `text` | 字符串 | 当前选区的文本；没有选区时为全文 |
| `line_ending` | 字符串 | 文档换行符：LF、CRLF 或 CR |
| `command` | 字符串 | 清单中本次命令的局部标识 |
| `options` | 对象 | 本条命令在清单中声明的选项 |

`options` 支持字符串、布尔值和整数。当前通过修改已安装的 `plugin.json` 并重新加载来更改选项，
暂未提供单独的选项表单。可以在标题中同时写中英文，方便命令搜索。

有多个非空选区时分别调用脚本，在一个撤销单元中应用所有结果；重叠选区会先合并。
所有选区都为空时处理全文。任何一次调用失败都会放弃整个操作，不会部分替换。

### 只读资源

清单可选字段 `"resources": ["dictionaries/words.txt"]` 声明 UTF-8 文本文件，脚本用
`read_resource("dictionaries/words.txt")` 获取内容。资源与脚本分开计量，导入、加载时统一校验，
加载后使用只读内存快照；更改文件需重新加载。读取未声明路径会报错。
路径必须在插件包内，不能读取任意文件、父目录或符号链接。此扩展保持原有无资源 API v1 插件兼容，
使用资源的新插件需要更新宿主。

脚本没有任意文件、网络、进程和编辑器内部对象的访问接口；不支持 `import` 和动态 `eval`。
插件使用源码分发，加载时编译并缓存 Grain 程序；没有字节码包和 VS Code VSIX 兼容接口。

## 执行与限制

- 导入、编译和执行在后台进行；一个窗口同时只运行一个插件任务。
- 单次操作输入、输出总量各不超过 8 MiB，最多 256 个选区。
- 脚本执行限制为 500 万操作、32 层函数调用，并检查 5 秒执行期限和取消信号。
  这些是脚本检查点上的限制，不是操作系统级的硬实时或进程内存隔离。
- 单脚本最多 2 MiB，单个资源最多 8 MiB，脚本和资源合计最多 16 MiB（解压后），最多 32 个资源。
- 清单最多 64 KiB，ZIP 压缩包最多 16 MiB；最多安装 64 个插件。资源不计入单脚本大小，但计入包总量。
- 包内路径必须是相对路径，拒绝目录穿越、符号链接和重复的 ZIP 路径。
- 执行期间文档被编辑、关闭或改成只读时，不会用旧结果覆盖文档。
- 插件禁用、重新加载或窗口关闭后，运行中的结果会被取消或丢弃。

## 维护与测试

`winc-plugin` 负责包校验、安装、加载与脚本执行；`winc-app/src/app/plugins.rs` 负责界面和文档事务；
`winc-app/src/keymap/plugins.rs` 将稳定命令标识接入现有命令面板与快捷键系统。

```sh
rtk cargo test -p winc-plugin
rtk cargo test -p winc-app plugins
```

API v1 仅开放文本处理。文件访问、网络调用、事件订阅和自定义界面需要后续单独设计接口和版本约定。
