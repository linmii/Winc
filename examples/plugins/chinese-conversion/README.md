# 简繁转换

导入本目录或 `chinese-conversion-1.0.0.zip`，在命令面板搜索“简体转繁体”或“繁体转简体”。
有选区时只转换选区，没有选区时转换全文，支持多选区、撤销和快捷键。
需要支持清单 `resources` 和 `read_resource` 的新版 Winc；旧版 API v1 宿主需要更新。

转换算法由 Rhai 脚本实现：从当前位置优先匹配最长词组，随后使用单字映射，未命中时保留原文。
保留换行、空白、英文、数字和 emoji；采用通用简繁字形，不进行台湾/香港用语转换。
简体的一字多义只能根据词典选择，不能保证人名、古文或任意文本的往返转换无损。

字典取自 OpenCC **1.1.9** 的未修改数据，采用每个词条的第一个候选：

- 来源：<https://github.com/BYVoid/OpenCC/tree/ver.1.1.9/data/dictionary>
- `STCharacters.txt` / `STPhrases.txt`：简体 → 繁体。
- `TSCharacters.txt` / `TSPhrases.txt`：繁体 → 简体。
- 许可证：`dictionaries/LICENSE`（Apache-2.0）。

只使用数据文件，不依赖 OpenCC 程序或库；这是独立实现，并非完整 OpenCC 配置流水线。
修改已安装插件的字典后执行“插件：重新加载”，新内容才会生效。
