Winc 自定义插件便携版（Windows x64）

解压整个压缩包到可写目录，运行 winc.exe。
如果旧版 Winc 仍在运行，请先退出旧版，或运行 winc.exe --new-instance。

已安装“去除重复行 / Remove Duplicate Lines”示例插件：
1. 打开文本，选中需要处理的内容；不选中则处理全文。
2. 打开命令面板，搜索“去除重复行”并执行。
3. 在“设置 → 快捷键”中搜索该命令，绑定自己的快捷键。
4. 可撤销整个处理操作；插件管理、导入和重新加载也位于命令面板。

插件位于 data/plugins/winc.remove-duplicate-lines，脚本和清单可直接编辑。
编写和导入其他插件的方法见 PLUGINS.md，无需另装 Rhai、Node.js 或 Python。
本包包含终端运行库、WSL 文件监视器和 Linux 远程服务程序。

这是 fork 的自定义构建。后续更新请使用同一 fork 的便携包并保留自己的 data 目录。
本包未使用 Windows 代码签名证书签名。
