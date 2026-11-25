# Fork 分支约定

`origin` 是本仓库 `linmii/Winc`，`upstream` 是源仓库 `DEEIX-AI/Winc`。

| 本仓库分支 | 用途 | 更新来源 |
| --- | --- | --- |
| `dev` | 上游开发分支的镜像，不提交自定义修改 | `upstream/dev` |
| `main` | 上游稳定分支的镜像，不提交自定义修改 | `upstream/main` |
| `codex/dev` | 自定义功能与上游开发版的整合 | `dev` 和功能分支 |
| `codex/main` | 自定义功能与上游稳定版的整合 | `main` 和已验证的自定义功能 |

`dev` 和 `main` 同步后应与各自的上游分支指向相同提交。文档、打包配置和功能修改
都只进入自定义分支。现有 `codex/plugin-system` 保留为插件功能分支。

当前插件功能只合入 `codex/dev`。`codex/main` 从 `main` 创建，暂不引入插件、
插件版打包工作流或本分支的其他自定义文件；后续按需要单独整合稳定版自定义功能。

## 同步上游

先确认工作区干净，再获取上游更新并快进镜像分支。若快进失败，先检查分叉原因，
不要直接强制推送或把自定义提交合进镜像分支。

```sh
rtk git fetch upstream
rtk git switch dev
rtk git merge --ff-only upstream/dev
rtk git switch main
rtk git merge --ff-only upstream/main
rtk git push origin dev main
```

然后分别整合到自定义分支，解决冲突并完成相关测试后推送：

```sh
rtk git switch codex/dev
rtk git merge dev
# 解决冲突、验证后：
rtk git push origin codex/dev

rtk git switch codex/main
rtk git merge main
# 解决冲突、验证后：
rtk git push origin codex/main
```

这些步骤需要手动执行；创建分支不会自动持续同步源仓库。

## 开发和稳定版整合

日常开发使用 `codex/dev`，也可以从它建立 `codex/<功能名>` 分支，完成后合回
`codex/dev`。`codex/main` 只引入与上游稳定版兼容、已经验证的自定义改动。

若功能分支包含尚未进入上游 `main` 的开发版提交，不要把整个功能分支或
`codex/dev` 合入 `codex/main`。应等待上游发布，或仅挑选自定义提交并适配稳定版，
避免把未发布的上游改动带入稳定分支。

## Windows 便携版

推送 `codex/dev` 或 `codex/plugin-system` 会触发
**Windows portable (custom)**，生成带插件的便携版 Actions 构建产物。
`codex/main` 暂不提供这个插件版打包工作流。
`dev`、`main` 保留上游的原始工作流。打包和下载方法见 [release/README.md](release/README.md)。
